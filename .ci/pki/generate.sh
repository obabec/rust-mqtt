#!/bin/bash
set -euo pipefail

# CA
openssl ecparam -name prime256v1 -genkey -noout -out ca-key.pem

openssl req -new -x509 \
  -key ca-key.pem \
  -days 3650 \
  -config cfg/ca.cnf \
  -out ca-cert.pem

# Server
openssl ecparam -name prime256v1 -genkey -noout -out server-key.pem

openssl req -new \
  -key server-key.pem \
  -subj "/CN=localhost" \
  -out server.csr

openssl x509 -req \
  -in server.csr \
  -CA ca-cert.pem \
  -CAkey ca-key.pem \
  -CAcreateserial \
  -sha256 \
  -days 825 \
  -extfile cfg/server.ext \
  -out server-cert.pem

# Client
openssl ecparam -name prime256v1 -genkey -noout -out client-key.pem

openssl req -new \
  -key client-key.pem \
  -subj "/CN=test-client" \
  -out client.csr

openssl x509 -req \
  -in client.csr \
  -CA ca-cert.pem \
  -CAkey ca-key.pem \
  -CAcreateserial \
  -sha256 \
  -days 825 \
  -extfile cfg/client.ext \
  -out client-cert.pem