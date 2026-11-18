---
cairn: change
id: one-address-reader
status: landed
created: 2026-11-18
---

# One address reader

## Why

Finding HIM-12 of the family's cli-family-analysis-2026-11-15 plan: addresses were read three ways. The Gmail backend split a header on commas, so `"Doe, John" <john@x.org>` listed as two senders; the read view took the first `<` of an entry, filing the valid `"Doe, John <john@x.org>" <john@x.org>` under `invalid.from`; `envelope list` and the composers read through mime-rs.

## What

mime-rs reads every address header: `email::address::parse` for Gmail, `parse_mailboxes` for each entry of the read view, which keeps two rules of its own (comments dropped, an address that is no valid one kept under `invalid`). Those two rules belong in mime-rs, other repositories being out of this change's reach.
