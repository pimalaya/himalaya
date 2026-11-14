---
cairn: delta
change: search-dates-can-be-relative
---

# Delta

## ADDED Requirements

### Requirement: Date values can be relative or instants
(search) A date clause SHALL take a day (`2026-11-14`, `2026/11/14`, `14-11-2026`, `14/11/2026`, `today`, `yesterday`, `<n> day|week|month|year[s] ago`, counted back in calendar terms) or an instant (`<n> minute|hour[s] ago`, an RFC 3339 date-time whose seconds and offset are optional). Days and offset-less instants SHALL be read in the local zone, `TZ` then the system's zone, falling back to UTC with one warning; now SHALL be taken once per query. `after` and `before` SHALL compare an instant to the second, strictly; `date` SHALL refuse an instant with `search-invalid`. A relative value SHALL be resolved at parse time. IMAP and any backend whose search knows days alone SHALL serve an instant on the chain of `and` from the root exactly, searching its day with the rest of the query and keeping the messages past or before the instant by their fetched dates, and SHALL refuse it under `or` or `not` with `search-unsupported`.

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

## MODIFIED Requirements

### Requirement: One query language
(search) The `envelope search` query SHALL parse into a single `SearchEmailsQuery` (filter plus sort), independent of the backend. The grammar covers the `date`, `after` and `before` date clauses, taking the date values of "Date values can be relative or instants", `after` and `before` both strict, the `from`, `to`, `cc` and `bcc` address clauses and `address` over all four, the `subject` and `body` text clauses, the `flag` clause over any flag name and the valueless `attachment` clause, combined with `not`, `and` and `or`, binding in that order, grouped with parentheses, and an `order by` list over date, from, to and subject. The sort comes last, introduced by the words `order by` outside any pattern, and lists at least one key. Every keyword of the grammar is a whole word. A pattern opening with a double quote is a quoted pattern, refused when unterminated; an unquoted pattern takes `\"`, `\\`, `\ `, `\(` and `\)` escapes. Blanks after `(` and before `)` are insignificant.

## REMOVED Requirements

None.
