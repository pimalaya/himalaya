---
cairn: log
change: every-jmap-get-names-what-it-did-not-find
landed: 2026-11-15
---

# Every JMAP get names what it did not find

Task H34 of the family's cli-alignment-2026-11-15 plan, left open by a-jmap-get-names-what-it-did-not-find: `jmap thread get`, `jmap identity get` and `jmap submission get` printed an empty list and exited 0 for an unknown id, the server's `notFound` reaching a log line alone.

## What landed

- src/error.rs: the codes `thread-not-found`, `identity-not-found` and `submission-not-found`, the mail domain's `message-` and `mailbox-` naming none of these RFC 8621 objects.
- src/jmap/backend.rs `not_found_errors`; src/jmap/thread/get.rs, src/jmap/identity/get.rs, src/jmap/submission/get.rs, query.rs, create.rs: an `errors` list, one entry per id not found, printed last in text, exit 3 when not empty; `jmap submission query` reports its chained get's `notFound` too, `jmap submission create` prints `[]`.
- Tests `a_thread_get_reports_each_id_not_found`, `an_identity_get_reports_each_id_not_found` and `a_submission_get_reports_each_id_not_found` over the scripted JMAP server, each failing on the old commands.

## Capabilities moved

- [commands](../spec/commands.md): *A command acting on several items lists what failed* names every JMAP get and the queries chaining one, with a scenario; the code table gains `thread-not-found`, `identity-not-found` and `submission-not-found`.
