---
cairn: tasks
change: the-sample-lists-values-in-brackets
---

# Tasks

- [x] config.sample.toml rewritten to the shape (from the example agreed on 2026-11-14)
- [x] src/config.rs: the sample test on the new variants, its helper in the test module; SECTIONS removed
- [x] Four sample mistakes (a bracket value, a commented key, a key of a replacing paragraph, the second `mailbox.alias.sent`) each fail the test, checked on a copy
- [x] `cargo test --all-features`; `cargo test --no-default-features --features rustls-ring,maildir`; clippy with the default features, `--all-features` and Maildir alone (the test carries no feature gate); fmt
- [x] CHANGELOG entry; fold the delta into cairn/spec/config.md; log entry; status landed
- [x] Commit
