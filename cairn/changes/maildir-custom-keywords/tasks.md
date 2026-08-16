---
cairn: tasks
change: maildir-custom-keywords
---

# Tasks

- [x] src/config.rs: add the `maildir.keywords.dovecot` and `maildir.keywords.header` fields, with a local `KeywordHeaderConfig` mirror so the config schema keeps compiling under every feature subset.
- [x] src/maildir/client.rs: convert the mirror into io-maildir's `KeywordHeader` and set both options on the inner client, covering the CLI and shared construction sites at once.
- [x] src/maildir/backend.rs: drop `parse_filename_flags` and `flag_from_char`, read flags through `MaildirFlags::with_dovecot` plus `extract_keywords_header`, and load the sidecar once per listing.
- [x] Document both options in config.sample.toml and the changelog.
- [x] Test the read path: defaults unchanged, slot letters resolved, both headers parsed, flag mapping round-trips.
- [x] Build/test/fmt/clippy, including the reduced feature builds.
- [x] Fold into cairn/spec/backends.md; log; land.
