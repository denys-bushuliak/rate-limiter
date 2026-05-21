use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use rate_limiter::FixedWindow;
use rate_limiter::LeakyBucket;
use rate_limiter::RateLimiter;
use rate_limiter::SlidingWindow;
use rate_limiter::TokenBucket;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::time::timeout;

#[derive(Parser, Debug)]
#[command(name = "rate_limiter_server")]
#[command(about = "A fast, lock-free rate limiter server", long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "3030")]
    port: u16,

    /// Algorithm to use for rate limiting
    #[command(subcommand)]
    algorithm: AlgorithmCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum AlgorithmCommand {
    /// Use the Fixed Window algorithm
    FixedWindow {
        /// Window size in seconds
        #[arg(short, long, default_value = "100")]
        window_size: u64,

        /// Maximum requests allowed in the window
        #[arg(short, long, default_value = "100")]
        capacity: u32,
    },
    /// Use the Leaky Bucket algorithm
    LeakyBucket {
        /// Rate at which the bucket leaks (requests per second)
        #[arg(short, long, default_value = "10")]
        rate_limit: u32,

        /// Maximum capacity of the bucket
        #[arg(short, long, default_value = "100")]
        capacity: u32,
    },
    /// Use the Sliding Window algorithm
    SlidingWindow {
        /// Window size in seconds
        #[arg(short, long, default_value = "100")]
        window_size: u64,

        /// Maximum requests allowed in the window
        #[arg(short, long, default_value = "100")]
        capacity: u32,
    },
    /// Use the Token Bucket algorithm
    TokenBucket {
        /// Rate at which tokens are added (requests per second)
        #[arg(short, long, default_value = "10")]
        rate_limit: u32,

        /// Maximum token capacity of the bucket
        #[arg(short, long, default_value = "100")]
        capacity: u32,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    dbg!(&args);

    let algorithm: Arc<dyn RateLimiter + Send + Sync> = match args.algorithm {
        AlgorithmCommand::FixedWindow {
            window_size,
            capacity,
        } => Arc::new(FixedWindow::new(
            Duration::from_secs(window_size),
            capacity as usize,
        )),
        AlgorithmCommand::LeakyBucket {
            rate_limit,
            capacity,
        } => Arc::new(LeakyBucket::new(capacity, rate_limit)),
        AlgorithmCommand::SlidingWindow {
            window_size,
            capacity,
        } => Arc::new(SlidingWindow::new(
            Duration::from_secs(window_size),
            capacity as usize,
        )),
        AlgorithmCommand::TokenBucket {
            rate_limit,
            capacity,
        } => Arc::new(TokenBucket::new(rate_limit, capacity)),
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    let listener = TcpListener::bind(addr).await?;
    println!("Server hyper started on http://{}", addr);

    loop {
        let (mut stream, _) = listener.accept().await?;
        let algorithm = Arc::clone(&algorithm);

        tokio::spawn(async move {
            const MAX_HEADER_SIZE: usize = 8192;
            let mut buf = Vec::with_capacity(1024);
            let mut chunk = [0u8; 1024];

            let read_result = timeout(Duration::from_secs(5), async {
                loop {
                    let n = stream.read(&mut chunk).await.map_err(|_| "Read error")?;

                    if n == 0 {
                        return Err("Client disconnected unexpectedly");
                    }

                    if buf.len() + n > MAX_HEADER_SIZE {
                        return Err("Header too large");
                    }

                    buf.extend_from_slice(&chunk[..n]);

                    let mut headers = [httparse::EMPTY_HEADER; 64];
                    let mut req = httparse::Request::new(&mut headers);

                    match req.parse(&buf) {
                        Ok(httparse::Status::Complete(offset)) => {
                            return Ok((req.path.unwrap_or("").to_string(), offset));
                        }
                        Ok(httparse::Status::Partial) => {
                            continue;
                        }
                        Err(_) => {
                            return Err("Invalid HTTP request");
                        }
                    }
                }
            })
            .await;

            match read_result {
                Ok(Ok((_path, _body_offset))) => {
                    if algorithm.allow() {
                        let response = format!("HTTP/1.1 200 OK\r\n\r\n");
                        let _ = stream.write_all(response.as_bytes()).await;
                    } else {
                        let response = "HTTP/1.1 429 Too Many Requests\r\n\r\n";
                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                }
                Ok(Err(err_msg)) => {
                    println!("Request error: {}", err_msg);
                    let response = "HTTP/1.1 400 Bad Request\r\n\r\n";
                    let _ = stream.write_all(response.as_bytes()).await;
                }
                Err(_) => {
                    println!("Request timed out (Slowloris protection)");
                    let response = "HTTP/1.1 408 Request Timeout\r\n\r\n";
                    let _ = stream.write_all(response.as_bytes()).await;
                }
            }
        });
    }
}
