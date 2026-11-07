---
cairn: tasks
change: tls-client-identity
---

# Tasks

- [x] Add `TlsIdentityConfig { cert, key }` and `TlsConfig::identity`, both paths shell-expanded.
- [x] Pass it to `Tls::identity` in `TlsConfig::into_tls`.
- [x] Test the parse: expansion, absence, and a certificate without its key refused.
- [x] Document it in config.sample.toml beside every `tls.cert`.
- [x] Check it against Dovecot requiring a client certificate.
- [ ] Replace the pimalaya-stream git patch with the release that carries `Tls::identity`.
