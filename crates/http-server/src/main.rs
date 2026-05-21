use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use rate_limiter::FixedWindow;
use rate_limiter::LeakyBucket;
use rate_limiter::RateLimiter;
use rate_limiter::SlidingWindow;
use rate_limiter::TokenBucket;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::timeout;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum Algorithm {
    FixedWindow,
    LeakyBucket,
    SlidingWindow,
    TokenBucket,
}

#[derive(Parser, Debug)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "3030")]
    port: u16,

    /// Rate limit in requests per second
    #[arg(short, long, default_value = "10")]
    rate_limit: f64,

    /// Algorithm to use for rate limiting
    #[arg(short, long)]
    algorithm: Algorithm,

    /// Window size for the rate limiter
    #[arg(short, long, default_value = "100")]
    window_size: String,

    /// Initial capacity of the rate limiter
    #[arg(short, long, default_value = "100")]
    capacity: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    dbg!(&args);

    let algorithm: Arc<Mutex<dyn RateLimiter>> = match args.algorithm {
        Algorithm::FixedWindow => {
            let window_size = args
                .window_size
                .parse()
                .map(Duration::from_secs)
                .expect("Window size should be a number");
            let max_requests = args.capacity as usize;
            Arc::new(Mutex::new(FixedWindow::new(window_size, max_requests)))
        }
        Algorithm::LeakyBucket => {
            let capacity = args.capacity as f64;
            let leak_rate = args.rate_limit;
            Arc::new(Mutex::new(LeakyBucket::new(capacity, leak_rate)))
        }
        Algorithm::SlidingWindow => {
            let window_size = args
                .window_size
                .parse()
                .map(Duration::from_secs)
                .expect("Windows size shoud be number");
            let max_requests = args.capacity as usize;
            Arc::new(Mutex::new(SlidingWindow::new(window_size, max_requests)))
        }
        Algorithm::TokenBucket => {
            let capacity = args.capacity as f64;
            let rate = args.rate_limit;
            Arc::new(Mutex::new(TokenBucket::new(rate, capacity)))
        }
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], 3030));
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
                            // Запит ще не повний, продовжуємо вичитувати з сокету
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
                    let mut allow = { algorithm.try_lock().unwrap() };
                    if allow.allow() {
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
