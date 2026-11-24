---
cairn: change
id: the-live-night-findings
status: active
created: 2026-11-24
---

# The live night findings

The live test night of 2026-11-23 (workspace .lanes/live/report.md) ran every himalaya command on Stalwart (IMAP, SMTP, Sieve, JMAP), Fastmail (IMAP, SMTP, JMAP), Gmail, Graph, iCloud, Maildir, mbox, posteo (read) and every pimdir account. c93b1b03 fixed the JMAP listing order and the performer choice. The pimdir failures seen through himalaya (a dropped sent copy, a submit parked after a rename) are neverest's (its cairn/changes/the-live-night-findings).

## What

1. **`mid:` search finds nothing over IMAP on Stalwart and iCloud.** `imap search --header Message-ID <id>` finds nothing there, `--text <local part>` finds the message, Fastmail matches. First check whether the value with its angle brackets (`HEADER Message-ID "<id>"`) matches on both servers; if it does, send it so. Otherwise mark `envelope.search.mid` `server` for IMAP, or fall back to `TEXT` on the local part with a client-side check of the Message-ID header.

## Not tested, for the next live run

Gmail settings writes (set then restore on the test account, owner to allow), vacation on Fastmail JMAP (set then restore), the google and microsoft live suites.
