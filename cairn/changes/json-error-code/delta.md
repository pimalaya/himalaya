---
cairn: change
change: json-error-code
---

# Delta

## ADDED Requirements

### Requirement: A JSON error carries a stable code
A failure a caller is expected to act on SHALL carry a stable code, printed under `--json` as a `code` string beside `error`, `sources` and `backtrace`. The code SHALL be found anywhere in the error chain, so added context does not hide it. A failure with no code SHALL print no `code` field. The plain output SHALL stay unchanged. Codes are kebab-case and never renamed; `error` stays free wording.

| Code | Raised when |
|---|---|
| `body-pending` | a listed message's body is not local yet (pimdir); a sync brings it |

#### Scenario: A body not downloaded yet
- GIVEN a pimdir item whose body is not local
- WHEN `himalaya --json message read` reads it
- THEN the command exits 1 and prints `{"code":"body-pending","error":"…","sources":[],"backtrace":null}`

## MODIFIED Requirements

### Requirement: pimdir is a reader and a producer, never the owner
`get_message` on an item whose body is not local SHALL fail with the code `body-pending`, the cue to sync, not a data-loss error; the item still lists.
