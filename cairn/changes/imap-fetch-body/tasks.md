---
cairn: tasks
change: imap-fetch-body
---

# Tasks

- [x] Cargo.toml: the `imap` feature enables `dep:base64` (already a dependency of `jmap`).
- [x] src/imap/fetch.rs: the `--body` flag, the `BodyExt { section: None, partial: None, peek: true }` item, `FetchedMessage.body` as Base64, a size line in the plain rendering.
- [x] Tests: the Base64 round trip of non-UTF-8 bytes; the item list built for `--body` alone, with other flags, and for no flag.
- [x] Build with the reduced feature set: `cargo build --no-default-features --features imap,smtp,rustls-ring`.
- [x] Manual provider test: the bytes of `imap fetch --body` equal `message read --raw` for the same UIDs, and `\Seen` is unchanged.
- [x] The CHANGELOG entry.
- [x] Fold the delta into [cairn/spec/commands.md](../../spec/commands.md); write [cairn/log/2026-09-25-imap-fetch-body.md](../../log/2026-09-25-imap-fetch-body.md).
