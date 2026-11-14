---
cairn: change
id: search-dates-can-be-relative
status: active
created: 2026-11-14
---

# Search dates can be relative

## Why

The `date`, `after` and `before` clauses of the query language take a calendar day alone, in four spellings (`2026-11-14`, `2026/11/14`, `14-11-2026`, `14/11/2026`, src/email/search/filter/parser.rs `naive_date`). A user cannot say `after 2 days ago`, as systemd's `--since` allows, nor give a time of day: `after $(date --iso-8601)` works, `after $(date --iso-8601=seconds)` is refused ([#528]). `pimdir search` already reads `today` and `yesterday` in its `date:` term, so the two languages also disagree on dates.

## What

### The grammar

A date clause takes a date value, which is one of:

- a day, as today: `2026-11-14` and the three other spellings;
- `today`, `yesterday`;
- `<n> <unit> ago`, `<unit>` one of `day`, `week`, `month`, `year` (singular or plural): a day, counted back from today in calendar terms (`1 month ago` on 2026-03-31 is 2026-02-28);
- `<n> <unit> ago`, `<unit>` one of `minute`, `hour` (singular or plural): an instant, counted back from now;
- an RFC 3339 date-time, its seconds and offset optional (`2026-11-14T09:30`, `2026-11-14T09:30:00+01:00`): an instant.

A multi-word value is written bare (`after 2 days ago`), the words `ago`, `today` and `yesterday` joining the grammar's keywords. Days and instants are read in the local zone, `TZ` then the system's zone, as `pimdir search` reads its `date:` terms (cairn/spec/search.md), falling back to UTC with one warning; an instant with an offset is read at it. Now is taken once per query.

The calendar units name a day rather than an instant on purpose: `after 2 days ago` keeps the mail sent after that whole day, as `after 2026-11-12` does today, so the relative words only spare the user the arithmetic. Precision below the day is asked for explicitly, with hours, minutes or a time of day.

`after` and `before` stay strict; on an instant they compare it to the second. `date` takes a day alone: `date 5 hours ago` is refused with `search-invalid`, the day holding that instant being said `date today` or `date yesterday`.

### The query and the backends

`SearchEmailsFilterQuery::{Date, AfterDate, BeforeDate}` carry a date value, a day or an instant, where they carry a `NaiveDate` today. A relative value is resolved at parse time, so no backend sees `ago`. The compared date stays the one each backend compares today: the `Date:` header, the received date on JMAP and Gmail (`partial`).

- Days: every backend serves them unchanged.
- Local evaluator (Maildir, mbox, pimdir `envelope search`): compares an instant to the envelope's date.
- Gmail: `after:` and `before:` already take Unix seconds; an instant is sent as is.
- JMAP: `after` and `before` take a UTCDate; the client-side re-check of the sent date, on the chain of `and` from the root, compares instants.
- IMAP: `SENTSINCE`, `SENTBEFORE` and `SENTON` know days alone (RFC 3501 §6.4.4). On the chain of `and` from the root, an instant `T` of day `D` is served exactly in two searches: the messages sent on `D` are searched with the rest of the query, their `ENVELOPE` dates fetched and kept when past (or before) `T`, then joined to the search of the days after (or before) `D`. Under `or` or `not`, where no residual holds, an instant clause is refused with `search-unsupported`, as the language refuses what a backend cannot answer exactly.
- Graph: whether KQL's `sent>` accepts a time of day is checked first; if not, Graph serves an instant as IMAP does, its page being sorted locally already.

### Help, spec, tests

The `QUERY` help gives the value forms with an example each. The spec gains the date values and the IMAP and Graph rule. Tests: the parser over each form and each refusal, with a fixed now; the local evaluator at the boundary second; the IMAP split against a test server (`SENTON` plus `SENTSINCE`); the Gmail and JMAP translations; the zone fallback.

## Scope / non-goals

No change to `pimdir search`'s own language, whose `date:` term follows pimdir SEARCH §8.1; adopting `<n> days ago` there is a pimdir draft question. No date ranges (`2026-01-01..2026-02-01`) in the shared language: `after` and `before` combined say it. No natural-language parsing beyond the forms above (`last monday`, `since noon`).

[#528]: https://github.com/pimalaya/himalaya/issues/528
