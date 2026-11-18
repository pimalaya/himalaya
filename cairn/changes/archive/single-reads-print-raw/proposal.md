---
cairn: change
id: single-reads-print-raw
status: landed
created: 2026-11-18
---

# Single reads print raw

## Why

Ruling of 2026-11-18 on the plan's "single-resource protocol reads" row (schema review A6, its remainder): ruling D18 made the protocol listings raw, but `msgraph messages get`, `gmail profile get`, `gmail drafts get`, `gmail messages get`, `gmail threads get`, the Gmail settings reads and `jmap vacation-response get` still printed through types that dropped what they did not model and renamed what they did (`email` for `emailAddress`).

## What

- Each of those reads prints the resource as the provider sent it, every property kept, its omissions its own; the Gmail settings sets print Gmail's answer to the `PUT` raw; `jmap vacation-response get` sends a raw `VacationResponse/get`.
- `gmail messages get` and `gmail threads get` no longer print a `headers` list of their own: the headers stay where Gmail puts them, under `payload.headers`, `--header` narrowing the text form, and the request under metadata.
- The text forms are unchanged.
- Breaking: the JSON of these commands takes the provider's shape.
