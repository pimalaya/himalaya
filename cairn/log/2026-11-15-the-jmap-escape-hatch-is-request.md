---
cairn: log
change: the-jmap-escape-hatch-is-request
landed: 2026-11-15
---

# The JMAP escape hatch is a request

Row M7 of the X3 report of the family's cli-alignment-2026-11-15 plan (finding F25).

## What landed

- src/jmap/request.rs (was src/jmap/query.rs): `jmap query` is `jmap request`, the family's name for the escape hatch of an HTTP API, its input (a methodCalls array, `--using`) and its `{methodResponses}` output unchanged.
- src/jmap/cli.rs: `jmap query` stays a hidden command running the same code; src/json_schema.rs registers the schema under `himalaya-jmap-request` and `himalaya-jmap-query`, a hidden clap alias leaving the old key naming no command for the registry check. `the_raw_request_is_jmap_request` checks the help, both names and both keys.
- CHANGELOG under Changed, not breaking; no MIGRATION.md entry, no MCP tool touched. The clap_complete generators still offer the hidden `query`, as they offer every hidden command.

## Capabilities moved

- [commands](../spec/commands.md): *The JMAP escape hatch is a request* added; *An option after the words is an option* names `jmap request`.
