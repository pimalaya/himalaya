---
cairn: log
change: the-tui-runs-on-himalaya
date: 2026-11-14
---

# The TUI runs on himalaya (himalaya's side)

The owner decided on 2026-11-14 that himalaya-tui runs himalaya's commands in-process over a git-pinned himalaya crate. Three things landed for it. Capabilities **packaging**, **commands** and **repl** moved.

- **src/lib.rs**: the module declarations and the architecture header, every module `pub`, no re-export; src/main.rs keeps `main` over `himalaya::{cli, error, session}`. The build script's `cfg(backend)` reaches the library as it reaches the binary. packaging.md's "Binary only" became "A binary, and a library for himalaya-tui": no semver promise beyond the CLI grammar and its `--json` output, a consumer pinning a git revision. README's Interfaces section says so. Making the modules public turned eight intra-doc links to private items into rustdoc warnings; they are plain code spans now.
- **`total` on `envelope list`**: each backend gained `list_envelopes_counted`, its `list_envelopes` delegating to it so the existing callers and tests stay untouched. IMAP reads the `EXISTS` of the `SELECT` it sends anyway, JMAP the `total` of its `Email/query` (`calculate_total` stated), Maildir counts the entries it listed, mbox the messages it read, pimdir asks the store's `count_items`, a local query. Gmail and Graph answer `null`. `EnvelopeListOutput` is shared with `envelope search`, which prints `total: null`, the family rule keeping absent keys as `null` rather than skipping them. The plain text output is unchanged.
- **`Session::keep_alive`**: `EmailClient::keep_alive` sends `NOOP` through the IMAP backend's new `keep_alive` and does nothing elsewhere; the session empties its unified slot when it fails. `Session::is_connected` says whether the unified client is built, for himalaya-tui to hand the terminal back around a command that connects. Both compile in every feature set, doing nothing where no unified client exists.

## Tests

- src/imap/backend.rs, against the scripted server: `a_listing_counts_the_mailbox_from_its_select` (3 messages, page 1 of 2, `total` 3), `a_keepalive_sends_noop`, `a_keepalive_on_a_closed_connection_fails`.
- src/maildir/backend.rs `a_page_counts_the_whole_mailbox` (page of 1, `total` 2).
- `cargo test`: 600 unit tests and every integration suite pass; clippy `-D warnings` clean over the 14 feature sets of the CI matrix.

Not covered by a test: the session emptying its slot after a failed keepalive (a three-line wrapper over the tested client call), and JMAP's `total` (no local JMAP server; it rides the `Email/query` the listing already sends).
