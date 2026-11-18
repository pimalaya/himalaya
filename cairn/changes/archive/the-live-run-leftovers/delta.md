---
cairn: delta
change: the-live-run-leftovers
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Date values can be relative or instants
A date clause SHALL take a day (`2026-11-14`, `2026/11/14`, `14-11-2026`, `14/11/2026`, its year written with four digits and its month and day with one or two, `today`, `yesterday`, `<n> day|week|month|year[s] ago`, counted back in calendar terms) or an instant (`<n> minute|hour[s] ago`, an RFC 3339 date-time whose seconds and offset are optional, its fraction of a second dropped). A year of another length SHALL be refused with `search-invalid`, so `15-11-26` is no date, the diagnostic saying a year takes four digits over the value given. Days and offset-less instants SHALL be read in the local zone, `TZ` then the system's zone, falling back to UTC with one warning when neither names one; now SHALL be taken once per query. `after` and `before` SHALL compare an instant to the second, strictly; `date` SHALL refuse an instant with `search-invalid`. A relative value SHALL be resolved at parse time. The local evaluator (Maildir, mbox, pimdir `envelope search`) SHALL compare an instant to the envelope's date; Gmail SHALL send it as Unix seconds in `after:` and `before:`; JMAP SHALL bound the received date with it in UTC and, on the chain of `and` from the root, re-check the sent date. IMAP SHALL serve an instant on the chain of `and` from the root exactly: it searches the days from the one before the instant's UTC day (`after`, `SENTSINCE`) or up to the second after it (`before`, `SENTBEFORE`), every message of that three-day band, whose `Date:` may fall on either side of the instant whatever its offset, searched with the rest of the query, its date fetched and kept only on the instant's side, before the page is cut; under `or` or `not` it SHALL refuse it with `search-unsupported`. Microsoft Graph, whose KQL `sent` compares days, SHALL refuse an instant with `search-unsupported` until a time of day is shown to work there.

#### Scenario: Two days ago is a day
- GIVEN the local date is 2026-11-14
- WHEN `himalaya envelope search after 2 days ago` runs
- THEN it searches as `after 2026-11-12` does

#### Scenario: Five hours ago is an instant
- GIVEN it is 2026-11-14T15:00 in the local zone, and an IMAP mailbox holding a message sent at 09:00 and one at 11:00 that day
- WHEN `himalaya envelope search after 5 hours ago` runs
- THEN only the message sent at 11:00 is listed

#### Scenario: An instant under or on IMAP
- WHEN `himalaya envelope search after 5 hours ago or from alice` runs on an IMAP account
- THEN it fails with `search-unsupported`, nothing searched

#### Scenario: A two-digit year
- WHEN `himalaya envelope search after 15-11-26` runs
- THEN it fails with `search-invalid`, its diagnostic pointing at `15-11-26` and saying a year takes four digits

### Requirement: The query's clauses are listed on -h
The first paragraph of the `QUERY` argument's help, which `-h` shows, SHALL name every clause, operator and the sort, comma separated, and point at `--help`, whose remaining paragraphs give each clause's syntax, examples and the precedence. The command's long help SHALL say how each backend matches text, and which date a date clause compares: the `Date:` header, but the received date on Gmail and on JMAP under `or` or `not`. Options MAY follow the query words; a word opening on a hyphen goes after `--`.

### Requirement: One TLS policy per account
An account's `tls` table SHALL apply to every network backend of that account (IMAP, SMTP, ManageSieve, JMAP, Gmail, Microsoft Graph), field by field: each of `provider`, `rustls.crypto`, `cert` and `client` a backend's own `tls` leaves unset SHALL take the account's, and one it sets SHALL win. A certificate an account trusts is so trusted on every transport, a JMAP download host and an SMTP submission after IMAP alike. The account table SHALL take the keys a backend's does, unknown ones refused. A `cert`, `client.cert` or `client.key` file, the account's or a backend's, that cannot be opened SHALL fail the command before it connects, the message naming the file and its path.

#### Scenario: One certificate for every transport
- GIVEN an account setting `tls.cert = "/etc/ca.pem"`, an `imap` block and an `smtp` block setting `smtp.tls.provider = "native-tls"`
- WHEN a command connects over IMAP, then over SMTP
- THEN both trust `/etc/ca.pem`, SMTP through native-tls

## REMOVED Requirements
