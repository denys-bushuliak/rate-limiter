### English

# Rate Limiter

This repository contains an HTTP server and a rate limiting library. The project is structured as a workspace with the crates http-server and rate-limiter.

## Documentation

For technical documentation, including architecture overview and detailed API references, see the [docs/](docs/) directory.

To run the server in release mode for maximum performance, use the following command. Since the program uses subcommands for the algorithms, you need to provide the desired algorithm as an argument.

Example for the Token Bucket algorithm:

```bash
cargo run --release -p http-server -- --port 3030 token-bucket --rate-limit 100 --capacity 500

```

To see all available algorithms and parameters:

```bash
cargo run --release -p http-server -- --help

```

## Load Testing with oha

The tool oha is used for load testing.

Installation via Cargo:

```bash
cargo install oha

```

Example for a test with 10,000 requests and 100 concurrent connections:

```bash
oha -n 10000 -c 100 http://127.0.0.1:3030/

```

## Tests and Coverage

To run the standard test suite for the entire workspace:

```bash
cargo test

```

We use cargo-llvm-cov to measure test coverage.

First, the tool must be installed system-wide:

```bash
cargo install cargo-llvm-cov

```

Then you can output the coverage to the terminal:

```bash
cargo llvm-cov

```

To generate a detailed HTML report showing which lines of code were covered by tests:

```bash
cargo llvm-cov --open

```

The path to the generated HTML file will be printed in the terminal at the end of the execution.
