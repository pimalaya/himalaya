---
cairn: change
id: the-list-frame-is-shared
status: landed
created: 2026-11-14
---

# The list frame is shared

> Cross-repo change, same id in pimalaya-cli (79c0d9e). Order: **pimalaya-cli** → **himalaya** (this side), Calendula and Cardamum after.

## Why

The owner decided on 2026-11-14 that the list frame and the `account capability` output belong to pimalaya-cli, so every binary prints a list, its item errors and an account's capabilities the same way. pimalaya-cli lifted them from Himalaya's src/listing.rs and src/account/capability.rs (pimalaya-cli cairn/changes/the-list-frame-is-shared), with two differences:

- Item errors print last: after the items and one blank line, one `Error:` line each, aligned among themselves, nothing when there is none. Himalaya printed a list's errors as `Error:` fields of the header block, and a write's as unaligned `Error:` lines right under its sentence.
- `AccountCapability::level` is a typed `CapabilityLevel`, serialized to the same strings.

The shared frame also widens the label column past `Next page:` when a longer label is present (`Incomplete:`, `History id:`), where Himalaya's overflowed it by one character.

Calendula and Cardamum expose `account capability` as a read-only MCP tool; Himalaya did not.

## What

- Cargo.toml patches pimalaya-cli by path (`../cli`) until its release.
- Every list prints through `pimalaya_cli::listing::{Listing, ListingPage}`; Himalaya's src/listing.rs keeps `counts` alone. `Backend::as_str` gives `Listing::new` the backend's name.
- `envelope list|search` and `mailbox list` push their errors onto `Listing::errors`; `flag add|set|remove` and `message copy|move|delete` print theirs through `error::write_item_errors` (pimalaya-cli's `ItemErrors::write_after`).
- `account capability` prints `pimalaya_cli::capability::AccountCapabilityOutput`; `capability::Support::level` and `StoreSupport::level` are `CapabilityLevel`. The JSON output and the schema key `himalaya-account-capability` are unchanged; its schema now lists the four levels.
- `himalaya mcp` lists `account_capability`, reading, whatever the account serves; the instructions name it.
- Spec: commands (text frame, item errors last, the capability output), mcp (the tool and the instructions).
