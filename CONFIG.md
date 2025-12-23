# Configuration

The gRPC file server uses a configuration directory located at `$HOME/.file_server/`.

## Directory Structure

```
$HOME/.file_server/
├── auth/
│   ├── server-cert.pem
│   ├── server-key.pem
│   ├── client-cert.pem
│   ├── client-key.pem
│   └── ca-cert.pem
└── config.json
```

## config.json

Create `config.json` with the following structure:

```json
{
  "server_bind_address": "0.0.0.0:50051",
  "server_connect_address": "192.168.1.149:50051",
  "upload_directory": "/home/user/file_server_uploads",
  "download_directory": "/home/user/file_server_downloads"
}
```

### Configuration Fields

- **`server_bind_address`**: The address the server binds to. Typically `0.0.0.0:50051` to listen on all interfaces.

- **`server_connect_address`**: The address the client connects to. This should be the actual IP address of the server machine (e.g., `192.168.1.149:50051`). Use `ip addr show` on the server to find your local IP.

- **`upload_directory`**: Directory where the server stores uploaded files. This directory will be created automatically if it doesn't exist.

- **`download_directory`**: Directory where the client saves downloaded files.

## Auth Directory

Place your TLS certificates in `$HOME/.file_server/auth/`:

- `server-cert.pem` - Server TLS certificate
- `server-key.pem` - Server private key
- `client-cert.pem` - Client TLS certificate
- `client-key.pem` - Client private key
- `ca-cert.pem` - CA certificate (for both server and client)

## Example Setup

```bash
# Create the config directory
mkdir -p ~/.file_server/auth

# Get your server's IP address
ip addr show

# Create config.json (replace 192.168.1.149 with your actual server IP)
cat > ~/.file_server/config.json << 'EOF'
{
  "server_bind_address": "0.0.0.0:50051",
  "server_connect_address": "192.168.1.149:50051",
  "upload_directory": "/home/$USER/uploads",
  "download_directory": "/home/$USER/downloads"
}
EOF

# Copy or symlink your certificates to ~/.file_server/auth/
# Then run:
cargo run --bin server
cargo run --bin tui-client
```

## Error Handling

If the configuration file or directory is missing, the application will display a helpful error message indicating what's missing and where to create it.
