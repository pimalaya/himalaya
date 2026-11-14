---
cairn: tasks
change: the-tui-runs-on-himalaya
---

# Tasks

- [x] src/lib.rs: the module declarations and the architecture header; src/main.rs over the library. Check the binary, every feature set and `cargo doc` build.
- [x] packaging.md "Binary only" follows (delta); README: the library is himalaya-tui's, pinned by git revision, with no semver promise beyond the CLI's.
- [x] `total` on the `envelope list` output and its JSON Schema; IMAP from `SELECT` `EXISTS`, JMAP `calculateTotal`, Maildir, mbox and pimdir counting, Gmail `null`; check Microsoft Graph's listing request and decide.
- [x] Tests: `total` per backend on its local or recorded fixtures; the schema registry check stays empty.
- [x] `Session::keep_alive`: `NOOP` on a kept IMAP connection, nothing otherwise, the slot emptied on failure; a test that a failed keepalive empties the slot and the next command rebuilds the client.
- [x] CHANGELOG under [Unreleased]: Added `total`; Added the library target for himalaya-tui.
- [ ] Push, then hand the revision to himalaya-tui's side of the change.
- [x] Fold the delta into packaging.md, commands.md and repl.md, write the log entry, set `landed`.
