---
cairn: change
id: the-api-is-unified
status: landed
created: 2026-11-12
---

# The API is unified

## Why

Since capability-shared-api and the-union-gains-its-verbs, the cross-protocol commands serve the union of what the backends can do, each backend declaring its level per capability, rather than the operations every backend shares. The owner renamed it on 2026-11-12: "unified API" says what it is, "shared API" suggested the least common denominator it no longer is.

## What

- "shared API" reads "unified API" across the code, its `--help` text (`Manage envelopes using the unified API.`), README.md, the v3 part of MIGRATION.md, the unreleased CHANGELOG entries, the spec, its test reports and the active changes; so do the phrases naming the API's parts: its commands, verbs, leaves, capabilities, writes, operations and outputs.
- Renamed requirements, their text otherwise unchanged: "Unified commands over a resolved transport", "The unified command tree is the same in every build", "Unified mailbox writes", "The important and junk flags are in the unified API" and "The auto-reply is in the unified API" (commands); "Unified operation set" and "Unified outputs carry no backend details" (backends); "Tools are the classified unified API" (mcp).
- The Gmail and Microsoft Graph client docs no longer call the API least-common-denominator.

## Scope / non-goals

No behaviour changes. "Shared" stays where it means shared by the code: the `src/shared` module, the `email` domain types every backend converts to, the query language, the layer and the client modules, pimalaya-cli's shared REPL. The log, landed changes, released CHANGELOG sections and the v1-to-v2 part of MIGRATION.md keep the words of their time. Calendula and Cardamum, which use the same phrase, are not touched.
