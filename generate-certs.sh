#!/bin/bash

mkdir -p auth

# Create CA
echo "Creating CA..."
openssl genrsa -out auth/ca-key.pem 4096
openssl req -new -x509 -days 365 -key auth/ca-key.pem -out auth/ca-cert.pem \
  -subj "/C=US/ST=State/L=City/O=MyOrg/CN=MyCA"

# Create server certificate with SAN
echo "Creating server certificate..."

# Create extensions file
cat > auth/v3.ext << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
IP.3 = 192.168.1.244
EOF

openssl genrsa -out auth/server-key.pem 4096
openssl req -new -key auth/server-key.pem -out auth/server-csr.pem \
  -subj "/C=US/ST=State/L=City/O=MyOrg/CN=localhost"
  
# Sign with extensions
openssl x509 -req -in auth/server-csr.pem \
  -CA auth/ca-cert.pem -CAkey auth/ca-key.pem \
  -CAcreateserial -out auth/server-cert.pem \
  -days 365 -sha256 -extfile auth/v3.ext \
  -set_serial 01

# Create Client Certificate  
echo "Creating client certificate..."
openssl genrsa -out auth/client-key.pem 4096
openssl req -new -key auth/client-key.pem -out auth/client-csr.pem \
  -subj "/C=US/ST=State/L=City/O=MyOrg/CN=client-alice"
openssl x509 -req -days 365 -in auth/client-csr.pem \
  -CA auth/ca-cert.pem -CAkey auth/ca-key.pem \
  -CAcreateserial -out auth/client-cert.pem -sha256 \
  -set_serial 02

# Cleanup
rm -f auth/*.csr auth/v3.ext

echo "Generated files in ./auth/"
