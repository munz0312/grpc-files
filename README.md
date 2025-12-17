# gRPC-files

A file server implementation using gRPC for client-server communication

## Usage

### Dependencies

[Rust](https://rust-lang.org/tools/install/)

[protoc](https://protobuf.dev/installation/) (Protocol Buffer Compiler)

Run the server:

```bash
cargo run --bin server
```

Run the client CLI in another terminal:

```bash
cargo run --bin client
```

You can try running the server on a different device, the server will be on port 50051

## Features

-- List file info
-- Upload/download files
-- Delete files

## Todo

-- Configuration files to set server IP address, upload and download directories
-- Maybe a TUI?
