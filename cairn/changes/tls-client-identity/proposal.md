---
cairn: change
id: tls-client-identity
status: active
created: 2026-11-06
---

# A client certificate for mutual TLS

## Why

A server that requires a client certificate refuses every Himalaya connection, whatever the password: there was no way to name one. Dovecot set up against a private CA (`ssl_server_request_client_cert`, the login refused without a verified certificate) is the case that prompted this.

## What

Every backend's `tls` block gains an optional `identity` table, `cert` and `key`, both PEM paths, expanded as the configuration is read like `tls.cert`. Both keys are required once the table is there, a certificate without its key being nothing that can be presented. `TlsConfig::into_tls` passes it to pimalaya-stream's `Tls::identity`, which presents it under both TLS providers.

## Depends on

pimalaya-stream's `client-identity` change. Until a release carries it, Cargo.toml patches pimalaya-stream to that branch.
