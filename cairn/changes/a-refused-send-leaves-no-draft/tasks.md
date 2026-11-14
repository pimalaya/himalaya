---
cairn: tasks
change: a-refused-send-leaves-no-draft
---

# Tasks

- [x] src/jmap/backend.rs `send_message`: destroy a created email on a `notCreated` submission, keep an existing one, warn on a failed destroy and on a failed submission request
- [x] Offline tests against a scripted JMAP server: the destroy sent and the refusal coded, an `alreadyExists` email kept, a failed destroy keeping the refusal
- [x] Clippy over the feature sets of tests.yml, `cargo test --all-features`, `cargo fmt`
- [x] Fold the delta into cairn/spec/backends.md, log, CHANGELOG
