# gRPC-files

A file server implementation using gRPC for client-server communication. Also features a simple TUI for the client using ratatui.

## Usage

### Dependencies

[Rust](https://rust-lang.org/tools/install/)

[protoc](https://protobuf.dev/installation/) (Protocol Buffer Compiler)

Refer to CONFIG.md, then CERTS.md to set up configuration files and certifates for TLS authentication.

Run the server:

```bash
cargo run --bin server
```

Run the TUI client in another terminal:

```bash
cargo run --bin tui-client
```

You can try running the server on a different device, the server will be on port 50051

## Features

- List file info
- Upload/download files
- Delete files

## Todo

