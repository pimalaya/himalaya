---
cairn: log
change: proxy-per-account
landed: 2026-09-29
---

# Proxy per account

Requested in #742: accounts that each go through their own proxy, or none. pimalaya-stream already tunnelled through SOCKS5 and HTTP CONNECT, but only as resolved from the environment, so one proxy served every account of a run.

**The libraries** moved first. io-imap, io-smtp and io-managesieve now connect with `(url, [domain,] opts)`, their TLS configuration, SASL mechanism and session options going into a `<Proto>ClientStdConnectOptions` beside a new `proxy`; io-jmap takes `(url, http_auth, opts)`; io-gmail and io-msgraph gain a `proxy` field on their existing options.

**`ProxyConfig`** (config.rs) is `url`, `username` and `password`, the password a `Secret`. An account `proxy` is handed to every network backend naming none at the moment the account is taken from the file, so each client reads its own block alone and `account check` sees the same resolved proxy. Absent both, `Proxy::System` keeps the environment behaviour.

Verified: 135 tests green, one new over the inheritance and the override; clippy on the default features and on each backend alone with rustls.

Spec updated: `config` (ADDED: "A proxy is set per account and per backend").
