---
cairn: change
id: protocol-commands-print-the-raw-payload
status: landed
created: 2026-11-15
---

# Protocol commands print the raw payload

## Why

Lines X11 and D26 of the family's cli-alignment-2026-11-15 plan, round X13. Ruling D18 (pimalaya-cli cairn/spec/output.md, *Protocol commands are raw-faithful*) landed for the listings in protocol-listings-print-the-raw-payload, but `jmap thread get`, `jmap identity get`, `jmap submission create`, `gmail labels list|get`, the Gmail settings lists, `msgraph mail-folders get` and `msgraph profile get` still re-serialized typed rows; `gmail labels get` and `msgraph mail-folders get` printed a list wrapper for one resource (schema review B14); `msgraph profile get` printed four keys where Cardamum and Calendula print Graph's user (X3 L20). Ruling D26: Himalaya gets the `gmail request` and `msgraph request` escape hatches Cardamum has, and `jmap request` takes and prints what Cardamum's does.

## What

- The commands above print the provider's payload as sent; `gmail labels get` and `msgraph mail-folders get` take one id and print the one resource.
- `gmail request` and `msgraph request` (method, path or URL, optional JSON body, `{response}`), copied from Cardamum's `gpeople request` and `msgraph request`; a shared `RawJsonOutput`.
- `jmap request <JSON>` takes one whole request object and prints `{response}`; the method-calls form stays as the hidden `jmap query`, its former name, with its `--using`, its `accountId` injection, its `{methodResponses}` output and its schema key.
