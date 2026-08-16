---
cairn: tasks
change: maildir-custom-keywords
---

# Tasks

- [x] io-maildir: add `MaildirFlags::with_keywords` (I/O-free) and resolve an entry's keywords on every client read path, the Maildir now being an argument; carry the result on `MaildirFullEntry::flags`.
- [x] src/config.rs: add the `maildir.keywords.dovecot` and `maildir.keywords.header` fields, with a local `MaildirKeywordHeaderConfig` mirror so the config schema keeps compiling under every feature subset.
- [x] src/maildir/client.rs: convert the mirror into io-maildir's `KeywordHeader` and set both options on the inner client, covering the CLI and shared construction sites at once.
- [x] src/maildir/backend.rs: drop `parse_filename_flags` and `flag_from_char`, read `MaildirFullEntry::flags` and map each one onto the shared `Flag`.
- [x] Document both options in config.sample.toml and the changelog, the `flag set` loss included.
- [x] Test what is left here: the flag mapping both ways, and the envelope carrying what the entry was read with. The keyword resolution itself is tested upstream.
- [x] Build/test/fmt/clippy, including the reduced feature builds.
- [x] Fold into cairn/spec/backends.md; log; land.
- [ ] Publish io-maildir 0.3 (prepared), then drop the `[patch.crates-io]` block standing in for it.
- [ ] himalaya-tui reads entries through the same API: bump it once 0.3 is out.
