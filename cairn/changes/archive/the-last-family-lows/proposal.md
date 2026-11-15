---
cairn: change
id: the-last-family-lows
status: landed
created: 2026-11-15
---

# The last family lows

## Why

Lines X9c, the queue row's `attempts` and D29 of the family's cli-alignment-2026-11-15 plan, round X13. The twins and Himalaya worded a duplicate collection differently, and Himalaya's Maildir path and its pimdir and Graph refusals each had their own sentence; pimalaya-cli's queue row declares `attempts` an integer from 0, which Himalaya printed as a signed one; and `-i` means `--interactive` across the family (D29) where Himalaya spent it on `--inline`.

## What

- Every `mailbox-duplicate` refusal reads "Cannot create mailbox `x`: it exists already", what the backend or the store answered kept in `sources`.
- `pimdir queue list` prints `attempts` as a `u64` (`uint64`, minimum 0, in the schema), as Cardamum does.
- `attachment list|download --inline` is long-only, `-i` a hidden short alias of it.
