---
cairn: log
change: the-v3-report-fixes
landed: 2026-11-26
---

# Graph's day refusal and `--stdout`'s one id

Two findings of the v3 provider reports of 2026-11-26, fixed before the 3.0.0 tag as the owner ruled.

## What landed

- M6 (msgraph.md): MIGRATION.md says Microsoft Graph compares days, so `date:2d` and `date:1w` are refused there and a day is written instead (`date:2026-11-24..`). The capability check's refusal of `date.instant` and the translation's refusal of an amount back from now give one reason, `GRAPH_DAYS` in src/unified/capability.rs: `` Microsoft Graph search compares days alone; give a day (`date:2026-11-24..`) or `today` ``. What a date term means is unchanged. Pinned by `graph_refuses_an_instant_before_connecting` (tests/capability.rs) and `an_instant_is_refused` (src/msgraph/search.rs), both failing against c3cdf686 on the old wordings; rechecked on the release build against the microsoft test account.
- F5 (imap-smtp-fastmail.md): `attachment download --stdout` given no id or several is refused as clap refuses a wrong number of values, `usage-invalid`, exit 2, before the client is built (`AttachmentDownloadCommand::check`, called from src/cli.rs). Pinned by two cases of tests/early_refusal.rs, which exited 1 uncoded against c3cdf686, the client built and its connection refused (`connect 127.0.0.1:1`).

## Capabilities moved

- [search](../spec/search.md): *Dates are read against one clock in the local zone* (Graph's one reason) and its scenario.
- [commands](../spec/commands.md): `attachment download --stdout` and the early refusals.
