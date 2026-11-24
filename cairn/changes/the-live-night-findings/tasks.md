---
cairn: tasks
change: the-live-night-findings
---

# Tasks

- [x] H1: reproduce `mid:` on Stalwart and iCloud with and without angle brackets (`imap search --header Message-ID`), journal the raw answers.
- [x] H2: send the form that matches, or mark `envelope.search.mid` `server` for IMAP, or fall back to `TEXT` plus a header check. Test: a scripted IMAP server answering only the bracketed form; live re-run on Stalwart and iCloud.
- [x] H3 (ruling, owner 2026-11-24): Stalwart 0.16 answers no `HEADER Message-ID` search in any form; IMAP keeps `envelope.search.mid` `full`, the gap documented as a provider quirk (cairn/spec/provider-quirks.md). No code change.
- [ ] Next live run: Gmail settings set and restore (owner to allow), vacation set and restore on fastmail-jmap, the google and microsoft suites.
