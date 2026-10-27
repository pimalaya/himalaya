---
cairn: change
id: machine-output-contract
status: landed
created: 2026-10-27
---

# Adopt the shared machine output contract

## Why

pimalaya-cli now owns the machine contract every binary prints (cli `machine-output-contract`, commit `55b6693`): the coded-error layer Himalaya introduced and calendula and cardamum copied, a schema for the failure object, schemas stamped with the contract version, piped `--json` delimited by newlines, and a check that every payload is a JSON object (what an MCP client takes as structured output).

## What

- src/error.rs keeps only the `ErrorCode` vocabulary; `CodedError` and `code_of` come from `pimalaya_cli::error`, and main.rs evaluates through `ErrorReport::<ErrorCode>::eval_coded`.
- src/json_schema.rs returns a `JsonSchemaRegistry` (bin `himalaya`, the crate version), registers `himalaya-error`, and tests that no payload root is anything but an object.
- pimalaya-cli is pinned at `55b6693` until its 0.3.0 release.

No payload changes: every registered root already is an object. Keys stay as they are; the camelCase switch is its own change (`json-keys-are-camel-case`).
