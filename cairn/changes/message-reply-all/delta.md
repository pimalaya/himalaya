---
cairn: delta
change: message-reply-all
---

## ADDED Requirements

### Requirement: Reply to all recipients
`message reply --all` (`-A`) SHALL keep the `To` of a plain reply and fill `Cc` with the `To` and `Cc` of the source, without the account's `email` and without any address already in `To`, each address once, compared ignoring case, display names kept. An explicit `--to` SHALL replace the derived `To`, the `Cc` then being deduplicated against it, and an explicit `--cc` SHALL replace the derived `Cc` whole. Without `--all`, `Cc` SHALL stay empty unless `--cc` fills it.
