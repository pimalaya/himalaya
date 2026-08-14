---
cairn: log
change: gmail-structured-json-output
landed: 2026-08-14
---

# Gmail structured JSON output

Every Gmail and Microsoft Graph command returning data now serializes it under `--json`, each with a JSON Schema registered. Fourteen commands moved: Gmail `messages get`, `drafts get`, `threads get`, `history list` and the nine `settings` readers, plus Microsoft Graph `message get`. Text output is unchanged everywhere except `messages get` and `threads get`, which now print the headers they were already fetching.

Fixes pimalaya/himalaya#730, where `gmail messages get --json` returned a human summary wrapped in a `message` string. Two corrections to the report, both worth keeping: `--header` was not inert, it reached Gmail as `metadataHeaders` and the headers came back, but the renderer dropped the payload; and the shared `envelope list` did serialize Gmail envelopes, so envelope data was never unreachable, only unreachable from that command. The severity was in the breadth, not the depth: the same `Message` wrapper covered thirteen more commands the reporter had not yet hit.

`GmailProfileOutput` already had the right shape and became the model. Two forms came out of it. Where a sibling `list` already serializes the backend resource, the `get` output is a `#[serde(transparent)]` newtype over that resource, so `get` and one row of `list` agree by construction; this covers the Gmail settings delegates, forwarding addresses, filters and send-as, and Graph `message get`. Where it does not, the fields are named: `GmailMessage`'s payload is a recursive MIME tree of base64 bodies that belongs on the raw path, and the five Gmail settings singletons do not derive `JsonSchema` in io-gmail, which would have meant changing a separate crate.

`gmail history list` was the sharpest case. Its published schema was a fake type, `HistoryOutput { message: String }` marked `#[allow(dead_code)]`, documenting the string wrapper as though it were the contract. It now reports the ids of the messages each record affects rather than the per-record counts the text summary shows, since driving an incremental sync is what the listing is for. That schema changing shape is the one documented contract this breaks; the other thirteen commands had no schema published.

`--header` now narrows the rendered headers under every format, not only `metadata`. Gmail honours `metadataHeaders` for that format alone, so the filter is applied to the response too, case-insensitively and preserving order and repeats. The request is unchanged, so the metadata path keeps saving bandwidth rather than fetching everything and discarding it. `threads get --header` was inert in exactly the same way and moved with it.

Also fixed while in the area: `gmail drafts get --format raw` fetched the raw message and discarded it, printing the draft summary. It now writes the RFC 5322 bytes like `messages get --format raw`.

Vec fields are serialized even when empty. schemars marks a `Vec` required whatever `skip_serializing_if` says, so skipping them would have published a schema the payload contradicts.

Deliberately not done: `threads get --format raw` stays broken. Gmail's `users.threads.get` has no raw format, so the request 400s; it predates this work, and the fix is a refusal at the CLI rather than an output type. Nothing in the request layer changed.

Verified: build, fmt and clippy clean; 88 tests pass, including four new ones for the header filter; the schema registry generates 69 schemas and the `required` lists match what is actually serialized.

Spec updated: commands (ADDED: Data commands serialize their data, Serialized collections are always present, Gmail header selection applies to every format, Raw message formats write bytes).
