---
cairn: tasks
change: tls-client-identity
---

# Tasks

- [x] Add `TlsClientConfig { cert, key }` and `TlsConfig::client`, both paths shell-expanded.
- [x] Pass it to `Tls::client` in `TlsConfig::into_tls`.
- [x] Test the parse: expansion, absence, and a certificate without its key refused.
- [x] Document it in config.sample.toml beside every `tls.cert`.
- [x] Check it against Dovecot requiring a client certificate.
- [x] Record it in CHANGELOG.md, fold the delta into cairn/spec/config.md, write the log entry and archive this change.
