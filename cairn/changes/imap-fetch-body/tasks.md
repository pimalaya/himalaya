---
cairn: tasks
change: imap-fetch-body
---

# Tasks

- [x] Cargo.toml: the `imap` feature enables `dep:base64` (already a dependency of `jmap`).
- [x] src/imap/fetch.rs: the `--body` flag, the `BODY.PEEK[]` item, `FetchedMessage.body` as octets serialized to Base64, the schema content annotations, a size line in the plain rendering.
- [x] Tests: the item list for `--body` alone, with `--flags` and with no flag; 8-bit octets kept intact by `from_items`.
- [x] Manual provider test (contributor, Yandex): the bytes of `imap fetch --body` equal `message read --raw` for the same UIDs, and `\Seen` is unchanged.
- [x] The CHANGELOG entry.
- [x] Fold the delta into [cairn/spec/commands.md](../../spec/commands.md); write [cairn/log/2026-09-30-imap-fetch-body.md](../../log/2026-09-30-imap-fetch-body.md).
