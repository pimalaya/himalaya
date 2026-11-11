---
cairn: change
id: one-trust-policy-per-account
status: draft
created: 2026-10-18
---

# One TLS trust policy per account, across IMAP and HTTP

## Why

Borrowed from an external engine's post-mortem: its IMAP and HTTP stacks trusted through two different stores, and a certificate accepted on one transport was refused on the other. Himalaya's host-injected transport (pimalaya-stream) avoids two TLS stacks, but not two policies.

Re-verified on 2026-10-18: trust is configured per backend. `imap.tls`, `smtp.tls`, `sieve.tls`, `jmap.tls`, `gmail.tls` and `msgraph.tls` each carry their own `TlsConfig` (provider, rustls crypto, `cert`) in src/config.rs, so an account behind a corporate or self-signed CA must repeat its `cert` for every transport, and one it forgets fails alone, often the second one used (SMTP after IMAP, a JMAP download host after the API host). The wizard's discovery used a fixed profile with no custom certificate at all; the wizard left on 2026-11-11 (the-wizard-leaves), and Himalaya runs no discovery any more.

## What

- An account-level `tls` section holding the provider, the crypto backend and the trusted certificates, inherited by every transport of the account; a transport's own `tls` section overrides it field by field.
- `account check` reports which trust each transport ended up with.

## Open questions

- Whether a per-transport override is still needed once the account level exists, or only kept for compatibility.
- Interaction with `native-tls`, where the platform store is the trust and `cert` adds to it.
