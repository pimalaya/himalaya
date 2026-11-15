---
cairn: tasks
change: live-provider-tests
---

# Tasks

- [x] tests/common.rs: config writer, binary runner with the token in its environment, JSON and shape assertions, sample sections, environment helpers
- [x] tests/fastmail.rs `anonymous`: JMAP `account check` and `envelope list` refused
- [x] tests/microsoft.rs `anonymous`: Graph, and IMAP with SMTP, refused
- [x] tests/google.rs `anonymous`: Gmail API and Gmail IMAP refused
- [x] tests/microsoft.rs `credentialed`: Graph, then Exchange IMAP once an app-only `account check` passed on the tenant
- [x] tests/google.rs `credentialed`: Gmail API and Gmail IMAP on a service account token
- [x] io-oauth as a dev-dependency, `cargo deny check` clean
- [x] .github/workflows/tests.yml: `live` matrix job, credentialed steps repository- and secret-guarded, `RUST_LOG=debug`, actionlint clean
- [x] Every suite run locally, and one debug-level run checked free of the tokens and the secret
- [x] cairn/spec/testing/imap-smtp-posteo.md removed with its README row
- [x] Fold the delta into cairn/spec/packaging.md, log
- [ ] The owner makes the `GOOGLE_SERVICE_ACCOUNT_KEY` and `MICROSOFT_*` org secrets visible to pimalaya/himalaya
