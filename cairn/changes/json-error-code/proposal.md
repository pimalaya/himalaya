---
cairn: change
id: json-error-code
status: landed
created: 2026-10-04
---

# A JSON error names a stable code when the caller has something to do

## Why

A front-end reading `--json` can tell one failure from another only by the wording of `error`, which is written for humans and changes with them. MOA matches `not downloaded yet|body not fetched` to show "still syncing" rather than "message lost" for a pimdir item whose body is not local. The first such failure deserves a contract.

pimalaya-cli's `ErrorReport` has no code, and shared codes across the six CLIs are a larger change of their own. The smallest stable mechanism lives here: a typed error carrying a code, found anywhere in the chain, printed beside the report.

## What

- `CodedError` (code and message) and `ErrorCode`, serialised in kebab-case. The first code is `body-pending`, raised by the pimdir backend's `get_message` for an item with no local body.
- `main` prints the failure through `error::eval`: pimalaya-cli's report, flattened, plus `code` when the chain carries a `CodedError`. A failure without one prints exactly as before, with no `code` field. The text output is unchanged.
