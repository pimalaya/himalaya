---
cairn: change
id: the-page-prints-above-the-items
status: landed
created: 2026-11-12
---

# The page prints above the items

## Why

every-list-prints-as-envelopes-do framed every list between two blocks of fields: `Account:` and what is listed above the items, `Page:`, `Next page:` and the other fields placing the answer under them. Having tried `mailbox` and `envelope`, the owner found the block under the items disturbing and asked on 2026-11-12 to merge it with the one above.

## What

- Every list prints one block of fields, then a blank line, then its items, and nothing under them: `Account:`, what is listed (`Mailbox:`, `Message:`, `Query:`), then `Page:` and `Next page:`, then `Incomplete:`, `History id:` and `Cancel:`.
- `listing::Listing` loses its `footer`: `page` and the commands adding `Incomplete:`, `History id:` and `Cancel:` push onto its `header`, in the order they are added.

## Scope / non-goals

The fields, their labels and their values do not change, nor does the `--json` output. On a page taller than the terminal, `Next page:` scrolls out of view with the header; the default page of `envelope list` (25) still fits a terminal of about 32 lines.
