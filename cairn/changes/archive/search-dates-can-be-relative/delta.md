---
cairn: delta
change: search-dates-can-be-relative
---

# Delta

## ADDED Requirements

### Requirement: Date values can be relative or instants
(search) A date clause SHALL take a day (`2026-11-14`, `2026/11/14`, `14-11-2026`, `14/11/2026`, its year written with four digits and its month and day with one or two, `today`, `yesterday`, `<n> day|week|month|year[s] ago`, counted back in calendar terms) or an instant (`<n> minute|hour[s] ago`, an RFC 3339 date-time whose seconds and offset are optional, its fraction of a second dropped). A year of another length SHALL be refused with `search-invalid`, so `15-11-26` is no date. Days and offset-less instants SHALL be read in the local zone, `TZ` then the system's zone, falling back to UTC with one warning when neither names one; now SHALL be taken once per query. `after` and `before` SHALL compare an instant to the second, strictly; `date` SHALL refuse an instant with `search-invalid`. A relative value SHALL be resolved at parse time. The local evaluator (Maildir, mbox, pimdir `envelope search`) SHALL compare an instant to the envelope's date; Gmail SHALL send it as Unix seconds in `after:` and `before:`; JMAP SHALL bound the received date with it in UTC and, on the chain of `and` from the root, re-check the sent date. IMAP SHALL serve an instant on the chain of `and` from the root exactly: it searches the days from the one before the instant's UTC day (`after`, `SENTSINCE`) or up to the second after it (`before`, `SENTBEFORE`), every message of that three-day band, whose `Date:` may fall on either side of the instant whatever its offset, searched with the rest of the query, its date fetched and kept only on the instant's side, before the page is cut; under `or` or `not` it SHALL refuse it with `search-unsupported`. Microsoft Graph, whose KQL `sent` compares days, SHALL refuse an instant with `search-unsupported` until a time of day is shown to work there.

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
- THEN it fails with `search-invalid`

## MODIFIED Requirements

### Requirement: One query language
(search) The `envelope search` query SHALL parse into a single `SearchEmailsQuery` (filter plus sort), independent of the backend. The grammar covers the `date`, `after` and `before` date clauses, taking the date values of "Date values can be relative or instants", `after` and `before` both strict, the `from`, `to`, `cc` and `bcc` address clauses and `address` over all four, the `subject` and `body` text clauses, the `flag` clause over any flag name, the valueless `attachment` clause and the `message-id` clause, taking a whole id with or without its angle brackets, combined with `not`, `and` and `or`, binding in that order, grouped with parentheses, and an `order by` list over date, from, to and subject. The sort comes last, introduced by the words `order by` outside any pattern, and lists at least one key. Every keyword of the grammar is a whole word. A pattern opening with a double quote is a quoted pattern, refused when unterminated; an unquoted pattern takes `\"`, `\\`, `\ `, `\(` and `\)` escapes. Blanks after `(` and before `)` are insignificant.

## REMOVED Requirements

None.
