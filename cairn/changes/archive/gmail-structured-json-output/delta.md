---
cairn: change
change: gmail-structured-json-output
---

# Delta

## ADDED Requirements

### Requirement: Data commands serialize their data
A command returning data SHALL hand the printer a dedicated output type implementing both `Display` and `Serialize`, and register its JSON Schema under the command's invocation key. `Message` is reserved for confirmations, since it serializes as a single `message` string and leaves `--json` unparseable. Where a sibling `list` already serializes a backend resource, the `get` output SHALL emit that resource verbatim through a transparent newtype, so one item read with `get` has the shape of one row of `list`. Where the wire type is unsuitable (a recursive MIME tree, a type carrying no schema), the output type SHALL name its fields instead.

### Requirement: Serialized collections are always present
An output field holding a collection SHALL be serialized even when empty, because the schema marks it required regardless and a skipped field would contradict the published schema.

### Requirement: Gmail header selection applies to every format
`gmail messages get --header` and `gmail threads get --header` SHALL narrow the rendered headers whatever the requested format. Gmail honours its `metadataHeaders` parameter under the metadata format alone and returns every header otherwise, so the narrowing SHALL also be applied to the response. Matching is case-insensitive, order and repeats are preserved, and passing no `--header` renders every header. Headers are read from the top-level payload part, where Gmail puts the RFC 5322 headers.

### Requirement: Raw message formats write bytes
A Gmail `get` command asked for the raw format SHALL decode the fetched message and write its RFC 5322 bytes through the shared byte writer rather than rendering a summary. This covers `messages get` and `drafts get`.

## MODIFIED Requirements

## REMOVED Requirements
