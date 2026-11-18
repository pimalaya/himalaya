---
cairn: delta
change: one-trust-policy-per-account
---

# Delta

## ADDED Requirements

### Requirement: One TLS policy per account
(config) An account's `tls` table SHALL apply to every network backend of that account field by field: each of `provider`, `rustls.crypto`, `cert` and `client` a backend's own `tls` leaves unset SHALL take the account's, and one it sets SHALL win.

## MODIFIED Requirements

### Requirement: account check covers every declared backend
(commands) Each row SHALL carry `tls`, `{provider, crypto, cert, clientCert}` as the backend's connection resolved it, `null` for a local backend.

### Requirement: Path keys expand as the configuration is read
(config) The account's `tls` paths expand as a backend's do.

## REMOVED Requirements

None.
