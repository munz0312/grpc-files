# Step CA Setup Guide

A guide to generating the certificates needed for TLS auth in this project

## Prerequisites

- Two machines on the same network (server and client)
- Root/sudo access

---

## Installation

### Server Machine

**Download and install Step CA server and CLI tools**

https://smallstep.com/docs/step-ca/installation/


### Client Machine

**Install only Step CLI tools (not the CA server)**

---

## CA Initialisation

### Initialise Certificate Authority (Server)

```bash
step ca init
```

Follow the instructions that are prompted.

**Created files:**
```
~/.step/
├── config/
│   └── ca.json              # CA configuration
├── certs/
│   ├── root_ca.crt          # Root CA certificate (public)
│   └── intermediate_ca.crt  # Intermediate CA certificate
└── secrets/
    ├── root_ca_key          # Root CA private key
    └── intermediate_ca_key  # Intermediate private key
```

---

## Configuration

### Extend Certificate Validity (Recommended)

Default certificate lifetime is 24 hours so change it to something longer:

```bash
# Edit CA configuration
$EDITOR ~/.step/config/ca.json
```

**Find and modify the `"claims"` section:**

```json
{
  "authority": {
    "claims": {
      "minTLSCertDuration": "1h",
      "maxTLSCertDuration": "8760h",
      "defaultTLSCertDuration": "720h"
    }
  }
}
```

### Start CA Server

```bash
step-ca $(step path)/config/ca.json
```

Step CA is now running on `https://<SERVER_IP>:9000`

---

## Certificate Generation

### Export CA Root Certificate (Server)

This certificate is needed by all clients to trust your CA. It will be used to by them to generate client certs.

Refer to CONFIG.md to see where to place the `auth/` directory.

```bash
cd ~/.file_server
mkdir -p auth

# Export CA root certificate
step ca root > auth/ca-cert.pem
```

**Distribute `auth/ca-cert.pem` to all clients.**

### Generate Server Certificate (Server)

```bash
cd ~/.file_server

# Generate server certificate with IP address
step ca certificate <SERVER_IP> auth/server-cert.pem auth/server-key.pem \
  --ca-url https://<SERVER_IP>:9000 \
  --root auth/ca-cert.pem \
  --san <SERVER_IP> \
  --san localhost \
  --not-after 720h
```

**Example:**
```bash
step ca certificate 192.168.1.149 auth/server-cert.pem auth/server-key.pem \
  --ca-url https://192.168.1.149:9000 \
  --root auth/ca-cert.pem \
  --san 192.168.1.149 \
  --san localhost \
  --not-after 720h
```

**Flags:**
- `--san`: Subject Alternative Name (add multiple for different addresses)
- `--not-after`: Certificate lifetime (720h = 30 days)

**Enter provisioner password when prompted.**

**Generated files:**
- `auth/server-cert.pem` - Server certificate
- `auth/server-key.pem` - Server private key

---

## Client Setup

### Transfer CA Certificate to Client

On server:
```bash
cd ~/.file_server/auth
python3 -m http.server 8000
```

On client:
```bash
cd ~/.file_server
mkdir -p auth
wget http://<SERVER_IP>:8000/ca-cert.pem -O auth/ca-cert.pem
```

You can also just use a USB drive or SSH to distribute the ca-cert.pem file.

### Generate Client Certificate (Client)

```bash
cd ~/.file_server

# Generate client certificate
step ca certificate client-<NAME> auth/client-cert.pem auth/client-key.pem \
  --ca-url https://<SERVER_IP>:9000 \
  --root auth/ca-cert.pem \
  --not-after 720h
```

**Example:**
```bash
step ca certificate client-laptop auth/client-cert.pem auth/client-key.pem \
  --ca-url https://192.168.1.149:9000 \
  --root auth/ca-cert.pem \
  --not-after 720h
```

**Naming convention:** Use descriptive names like `client-laptop`, `client-desktop`, `client-phone` to identify different devices. The name becomes the certificate's Common Name (CN) and helps with tracking which device is connecting.

**Enter provisioner password when prompted.**

**Generated files:**
- `auth/client-cert.pem` - Client certificate
- `auth/client-key.pem` - Client private key

---

## Certificate Renewal

Server and client certificates expire after their validity period (default: 30 days).

CA certificates expire after 10 years.

### Renew Server Certificate (Server)

```bash
cd ~/.file_server

step ca renew auth/server-cert.pem auth/server-key.pem \
  --ca-url https://<SERVER_IP>:9000 \
  --root auth/ca-cert.pem \
  --force
```

**Restart your server application after renewal.**

### Renew Client Certificate (Client)

```bash
cd ~/.file_server

step ca renew auth/client-cert.pem auth/client-key.pem \
  --ca-url https://<SERVER_IP>:9000 \
  --root auth/ca-cert.pem \
  --force
```

### Common Issues

You'll need to make sure your server machine will actually allow traffic from TCP via the ports being used (eg 50051 for gRPC, 9000 for the CA server) so set up rules in whatever way is appropriate for your OS.

