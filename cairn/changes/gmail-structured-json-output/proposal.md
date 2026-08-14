---
cairn: change
id: gmail-structured-json-output
status: landed
created: 2026-08-14
---

# Gmail structured JSON output

## Why

Reported as pimalaya/himalaya#730: `gmail messages get --json` returns `{"message":"Id: 19ffb3...\nThread: ...\n"}`, a human summary wrapped in a JSON string, so nothing can be parsed out of it and `--header` appears to do nothing.

The report understates one thing and overstates another. `--header` is not inert: it reaches Gmail as `metadataHeaders` and the headers come back; the renderer drops the payload. And the shared `envelope list` does serialize Gmail envelopes properly, so subjects and senders were never unreachable, just unreachable from that command.

The defect is one renderer handing `printer.out` a `Message`, which serializes as a single `message` string. The same shape covers twelve more Gmail commands and one Microsoft Graph command, all of which the reporter could have hit next. `gmail history list` is the worst of them: its published JSON Schema documented the string wrapper as if it were the contract.

## What

Every command returning data serializes it, with a JSON Schema registered. `GmailProfileOutput` already had the right shape and becomes the model: a `pub(crate)` output type deriving `Serialize + JsonSchema`, kebab-case, with a hand-written `Display` carrying the text rendering unchanged.

Two forms, chosen by whether a sibling `list` already serializes the resource. Where it does (the Gmail settings readers, Microsoft Graph `message get`), the output is a `#[serde(transparent)]` newtype over the backend resource, so `get` and one row of `list` agree by construction. Where it does not, or the wire type is unsuitable, the fields are written out: `GmailMessage`'s payload is a recursive MIME tree carrying base64 bodies and belongs on the raw path, and the five Gmail settings singletons do not derive `JsonSchema` in io-gmail.

`--header` also starts narrowing the rendered headers under every format, not only `metadata`. Gmail honours `metadataHeaders` for that format alone, so the filter is applied to the response as well.

## Scope / non-goals

No change to what is requested from Gmail. The fix is in the renderer, and `metadataHeaders` still goes out under the metadata format so that path keeps saving bandwidth.

Vec fields are always emitted rather than skipped when empty, because schemars marks a `Vec` required regardless of `skip_serializing_if` and the schema would otherwise disagree with the payload.

`threads get --format raw` is left broken. Gmail's `users.threads.get` has no raw format, so the request 400s. It predates this work and the fix is a CLI-level refusal, not an output type.
