---
cairn: log
change: the-live-run-leftovers
landed: 2026-11-18
---

# The live run leftovers

The observations of the live provider run of 2026-11-18 (cairn/spec/testing/imap-smtp-fastmail.md, jmap-fastmail.md J4, imap-smtp-specific-fastmail.md, maildir-specific.md).

## What landed

- src/email/search/filter/parser.rs: a short-year date reads ``a year takes four digits, as in `2026-11-15` `` over the value.
- src/tls.rs: a certificate or key file that cannot be opened fails before the connection, naming its path.
- src/shared/envelope/search.rs: `envelope search --help` names the received-date exceptions (Gmail, JMAP under `or` or `not`).
- cairn/spec/testing/provider-test-plan.md: `imap raw -- …`, `smtp raw -- …`, the Maildir surface with `--maildir`.
- Tests, each failing on 3b9f9127: `a_two_digit_year_is_refused` (the diagnostic), tests/tls_cert.rs.
- CHANGELOG (Fixed).

## Capabilities moved

- [search](../spec/search.md): *Date values can be relative or instants*, *The query's clauses are listed on -h*.
- [config](../spec/config.md): *One TLS policy per account*.
