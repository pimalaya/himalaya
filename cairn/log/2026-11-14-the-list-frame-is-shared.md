---
cairn: log
change: the-list-frame-is-shared
landed: 2026-11-14
---

# The list frame is shared

Himalaya prints its lists, their item errors and `account capability` through pimalaya-cli (79c0d9e, patched by path in Cargo.toml until its release), which lifted them from Himalaya's own src/listing.rs and src/account/capability.rs.

- Every list builds a `pimalaya_cli::listing::Listing`; src/listing.rs keeps `counts` alone, and `Backend::as_str` names the backend in `Account:`. A label longer than `Next page:` now widens the field column rather than overflowing it: `Incomplete:` (`pimdir search`, `pimdir tag`) and `History id:` (`gmail history list`).
- Item errors print last: `envelope list|search` and `mailbox list` after the items, `flag add|set|remove` and `message copy|move|delete` after the sentence, one blank line before, one `Error:` line each, aligned among themselves (`error::write_item_errors` for the writes). They printed as `Error:` fields of the header block, and as unaligned lines right under a write's sentence. The `--json` output is unchanged.
- `account capability` prints `pimalaya_cli::capability::AccountCapabilityOutput`, `capability::Support::level` and `StoreSupport::level` typed `CapabilityLevel`. The JSON output and the `himalaya-account-capability` key are unchanged; its schema now lists the four levels as an enum.
- `himalaya mcp` lists `account_capability`, a reading tool whatever the flags and the account (readOnlyHint true, destructiveHint false, openWorldHint false), and its instructions name it, as Calendula and Cardamum do.

Tested: the byte-exact tests of `envelope list` (now two errors, aligned), `mailbox list`, `flag add`, `message delete`, `pimdir search` and tests/exit.rs take the new layout, each failing on the old one; tests/mcp.rs `account_capability_says_what_the_account_serves` lists and calls the tool, failing before it existed. `cargo test --all-features`, clippy `-D warnings` over every feature set of tests.yml plus `--no-default-features`, and the Maildir-only tests.

Spec: commands ("An item that failed prints last" added, replacing "An item that failed prints in the header block"; "Text output is plain text" and "An account lists what it serves" modified), mcp ("The server tells the agent how to use it" and "Tools are the classified unified API" modified).

The errors and the list frame are unreleased, so the CHANGELOG amends their [Unreleased] entries rather than adding a breaking one. Left open: the path patch of pimalaya-cli becomes a version once it is released.
