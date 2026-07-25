---
cairn: tasks
change: wizard-auth-and-mailbox-aliases
---

- [x] Collapse the discovery entries to one per service, carrying an `AuthCaps` capability set instead of a per-method row
- [x] Add the service-specific auth prompt: SASL mechanism for IMAP + SMTP, HTTP scheme for JMAP, skipped when single
- [x] Fold OAuth into the API-token credential prompt (keyrings plus brokers, brokers only when OAuth advertised); remove the OAuth dead-end note and loop
- [x] Test IMAP then ask whether SMTP shares credentials, re-running the SASL prompt otherwise, then test SMTP
- [x] Drop the account-name prompt; derive the name from the input
- [x] Pre-fill `mailbox.alias.*`: JMAP roles live, Gmail/Graph fixed ids, IMAP `INBOX`
- [x] Extend the shared cli token picker to combine keyrings and brokers behind an `oauth` gate
- [x] Update config.sample.toml and CHANGELOG
