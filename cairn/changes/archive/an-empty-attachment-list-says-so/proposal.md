---
cairn: change
id: an-empty-attachment-list-says-so
status: landed
created: 2026-11-12
---

# An empty attachment list says so

## Why

Since every-list-prints-as-envelopes-do, an unpaged list without items prints its header alone. On `attachment list`, a message without attachments then printed `Account:`, `Mailbox:` and `Message:` and stopped, which reads as if the list were cut. On 2026-11-12 the owner asked for the plain output to state it, the `--json` `[]` being explicit enough.

## What

- `listing::Listing` gains `empty`, a line printed in place of the items when there are none, a blank line under the header as the items would be.
- `attachment list`, and `attachment download` which prints the same list, set it to `No attachments`.

## Scope / non-goals

The `--json` output does not change. The other unpaged lists keep printing their header alone when empty.
