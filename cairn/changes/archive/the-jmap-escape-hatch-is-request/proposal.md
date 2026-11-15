---
cairn: change
id: the-jmap-escape-hatch-is-request
status: landed
created: 2026-11-15
---

# The JMAP escape hatch is a request

## Why

Row M7 of the X3 report of the family's CLI alignment round (/home/soywod/code/pimalaya/.github/cairn/changes/cli-alignment-2026-11-15/reports/x3-2026-11-15.md, finding F25). pimalaya-cli's cairn/spec/commands.md names `request` the namespace command of an HTTP API sending one request, beside `raw` for a line protocol, and Cardamum has `jmap request`, `msgraph request` and `gpeople request`. Himalaya's JMAP escape hatch was `jmap query` (src/jmap/query.rs): not a typed query but an arbitrary methodCalls array sent as one JMAP request, the raw `methodResponses` printed, the name read as `Email/query` or `Mailbox/query` beside `jmap email query` and `jmap mailbox query`.

## What

`jmap query` becomes `jmap request` (src/jmap/request.rs, `JmapRequestCommand`, `JmapRequestOutput`), its input and output unchanged. `jmap query` stays a hidden command running the same code, as Calendula's `gcal instances` does, so its schema key `himalaya-jmap-query` stays beside the new `himalaya-jmap-request`: a hidden clap alias would leave that key naming no command, which pimalaya-cli's `JsonSchemaRegistry::check` reports and `every_command_is_described` refuses. Nothing breaks, so no MIGRATION.md entry; no MCP tool is involved, the server exposing no protocol command.

The input stays a methodCalls array with `--using`, where Cardamum's takes a whole request object, and the output stays `{methodResponses}`, where Cardamum's is `{response}`: aligning them would break, left to the owner.
