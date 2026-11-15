---
cairn: change
id: a-body-clause-reads-the-local-body
status: landed
created: 2026-10-30
---

# A pimdir body clause reads the local body

## Why

The pimdir stable-release review of 2026-10-30 (report-himalaya.md, should-fix 1) verified that a `body` clause never matches on the pimdir backend: both search paths hand the shared matcher an empty body (`eval::matches_filter(&envelope, &[], filter)`, src/pimdir/backend.rs:298 and :331). On a store holding three local bodies containing "See attached.", `envelope search -m imap/INBOX body attached` returned nothing with exit 0, and `not body attached` returned every message. The doc comment at src/pimdir/backend.rs:226-229 and the draft a-search-says-what-it-covered say only unfetched bodies miss, which is wrong.

This is phase 0 of the approved pimdir search plan (pimdir-search): the shared `envelope list` keeps this in-memory path until the v3 query moves to the SEARCH.md language.

## What

- When the filter holds a `body` clause, the scan reads each candidate's local body (`item.object` set) from the blob store and hands it to the matcher, as Maildir, m2dir and mbox do. A filter without a body clause reads no body.
- An item with no local body, or whose blob is gone, is matched with an empty body, as before: a body clause cannot match it, and that is what pimdir-search's coverage line will say.
- The doc comment and spec/search.md say what a pimdir body clause covers.
