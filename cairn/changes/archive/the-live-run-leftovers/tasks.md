---
cairn: tasks
change: the-live-run-leftovers
---

# Tasks

- [x] src/email/search/filter/parser.rs: `dated`, `short_year`, the count of `ago` read whole; `a_two_digit_year_is_refused` asserts the diagnostic, failing on dd9c6b12.
- [x] src/tls.rs: `into_tls` refuses an unreadable file by path; its callers propagate; tests/tls_cert.rs failing on dd9c6b12.
- [x] src/shared/envelope/search.rs: the help names the received-date exceptions.
- [x] cairn/spec/testing/provider-test-plan.md: `--` for the raw commands, the Maildir bullet.
- [x] Fold into cairn/spec/search.md and config.md, CHANGELOG.
