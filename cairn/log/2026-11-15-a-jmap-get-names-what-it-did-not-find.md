---
cairn: log
change: a-jmap-get-names-what-it-did-not-find
landed: 2026-11-15
---

# A JMAP get names what it did not find

Task H30 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (jmap-fastmail.md, O1): `jmap email get <bogus>` and `jmap mailbox get <bogus>` printed an empty list and exited 0, the server's `notFound` reaching a log line alone.

## What landed

- src/jmap/backend.rs `get_errors`; src/jmap/email/get.rs, query.rs, src/jmap/mailbox/get.rs, query.rs: an `errors` list, one entry per id not found (`message-not-found`, `mailbox-not-found`) and per object that did not deserialize (`message-unreadable`, `mailbox-unreadable`), printed last in text, exit 3 when not empty.
- Tests `an_email_get_reports_each_id_not_found` and `a_mailbox_get_reports_each_id_not_found` over the scripted JMAP server, both failing on the old commands.
- Left open: `jmap thread get`, `identity get` and `submission get` still log their `notFound` alone.

## Capabilities moved

- [commands](../spec/commands.md): *A command acting on several items lists what failed* names the JMAP gets and queries among the protocol commands printing `errors`, with a scenario; the `mailbox-not-found` and `message-not-found` rows.
