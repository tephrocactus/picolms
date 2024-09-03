#!/bin/bash
set -e
cd $(dirname "${BASH_SOURCE[0]}")

openssl genrsa -out api-ca.key 4096
openssl req -x509 -new -nodes -key api-ca.key -subj "/CN=api-ca" -days 10000 -out api-ca.crt
openssl genrsa -out api-server.key 4096
openssl req -new -key api-server.key -out api-server.csr -config api-server-csr.conf
openssl x509 -req -in api-server.csr -CA api-ca.crt -CAkey api-ca.key -CAcreateserial \
    -out api-server.crt -days 10000 -extensions v3_ext -extfile api-server-csr.conf -sha256
