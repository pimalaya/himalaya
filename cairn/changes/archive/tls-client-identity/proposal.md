---
cairn: change
id: tls-client-identity
status: landed
created: 2026-11-06
---

# A client certificate for mutual TLS

## Why

A server that requires a client certificate refuses every Himalaya connection, whatever the password: there was no way to name one.

## What

Every network backend's `tls` block gains an optional `client` table, `cert` and `key`, both PEM paths, expanded as the configuration is read like `tls.cert`. Both keys are required once the table is there. `TlsConfig::into_tls` passes it to pimalaya-stream's `Tls::client`. `tls.cert` keeps its name.
