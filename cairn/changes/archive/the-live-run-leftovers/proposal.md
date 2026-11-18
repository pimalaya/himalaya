---
cairn: change
id: the-live-run-leftovers
status: landed
created: 2026-11-18
---

# The live run leftovers

## Why

The observations of the live run of 2026-11-18: `after 15-11-26` was refused with `found -11-26, expected something else or space after the number`; a missing `tls.cert` failed with `No such file or directory (os error 2)`, no path named; `envelope search --help` said every date clause reads the `Date:` header, though Gmail and JMAP under `or` or `not` compare the received date (J4); provider-test-plan.md showed `raw '<cmd>'` and no Maildir surface, the reports finding `--` and `--maildir` needed.

## What

- The short-year refusal names the four digits, over the value given: a date clause tries the short-year shape before its labelled value, which would hide the refusal, and `<n> <unit> ago` reads its whole count first, so it no longer fails past the value's first digits.
- A certificate or key file that cannot be opened fails before the connection, naming its path. The file is read again by pimalaya-stream; naming the path there too belongs upstream.
- The search help names the received-date exceptions.
- The test plan shows `imap raw -- …`, `smtp raw -- …` and the Maildir surface with `--maildir`.
