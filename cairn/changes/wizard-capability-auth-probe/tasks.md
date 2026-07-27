---
cairn: tasks
change: wizard-capability-auth-probe
---

- [x] Add `available_auth_mechanisms` to io-imap's coroutine core (rfc3501::capability), reusing pimalaya-stream's `SaslMechanism` (ordered by preference, LOGIN last, honours LOGINDISABLED); pull pimalaya-stream minimally as a non-optional dep so it is not TLS-provider gated
- [x] Add a wizard IMAP capability probe: unauthenticated connect that reads CAPABILITY and returns the advertised mechanisms
- [x] Offer only probed mechanisms in the discovered path; log and fall back to the full list on probe failure
- [x] Probe and pick a mechanism in the manual path instead of hardcoding PLAIN
- [x] Keep SMTP keyed on discovery (EHLO, not the IMAP probe)
- [x] Bound discovery with `compose_all_within` in io-pim-discovery and call it from the wizard
- [x] Update CHANGELOG in himalaya, io-imap and io-pim-discovery
