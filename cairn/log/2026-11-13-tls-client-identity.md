---
cairn: log
change: tls-client-identity
landed: 2026-11-13
---

# A client certificate for mutual TLS

A server requiring a client certificate refused every Himalaya connection, whatever the password: no key named one. Every network backend's `tls` block takes a `client` table, `cert` and `key`, PEM paths expanded as the configuration is read like `tls.cert`. Both are required once the table is there. `TlsConfig::into_tls` passes it to pimalaya-stream 0.4's `Tls::client` where it passed `None`; `tls.cert` keeps its name and still maps to `Tls::server`, so no configuration changes.

Tested by the parse: expansion, absence, and a certificate without its key refused. Checked by hand, before the rebase onto pimalaya-stream 0.4, against Dovecot requiring a client certificate from a private CA, under rustls and native-tls: refused without the certificate, the handshake completing with it. The mapping into `Tls::client` is unchanged since.

Spec: config ("A client certificate per backend" added; the path keys amended).
