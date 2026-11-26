---
cairn: tasks
change: the-live-night-findings
---

# Tasks

- [x] H1: reproduce `mid:` on Stalwart and iCloud with and without angle brackets (`imap search --header Message-ID`), journal the raw answers.
- [x] H2: send the form that matches, or mark `envelope.search.mid` `server` for IMAP, or fall back to `TEXT` plus a header check. Test: a scripted IMAP server answering only the bracketed form; live re-run on Stalwart and iCloud.
- [x] H3 (ruling, owner 2026-11-24): Stalwart 0.16 answers no `HEADER Message-ID` search in any form; IMAP keeps `envelope.search.mid` `full`, the gap documented as a provider quirk (cairn/spec/provider-quirks.md). No code change.
- [x] Live night 2 (2026-11-24): vacation set and restore on fastmail-jmap passed; the google and microsoft suites pass (11 and 11) once their four credentialed `account check` tests read `accounts[0]` (stale since the `accounts` wrapper; test-only fix in tests/google.rs and tests/microsoft.rs).
- [x] ~~Gmail settings set and restore (vacation, a filter, a signature)~~ Moved 2026-11-26 to the live checks of cairn/spec/testing/gmail-specific.md, NOT RUN there: still blocked, the rc Google token lacks `https://www.googleapis.com/auth/gmail.settings.basic` (updateVacation answers 403 insufficient scopes). Add the scope to the ortie Google account, re-consent, re-run.
