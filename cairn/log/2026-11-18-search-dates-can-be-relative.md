---
cairn: log
change: search-dates-can-be-relative
landed: 2026-11-18
---

# Search dates can be relative

The proposal of 2026-11-14 ([#528]), with the two-digit year the first batch of 2026-11-18 found (`after 15-11-26` read as the year 15).

## What landed

- src/email/search/filter/parser.rs: `query_at(now)`, the date values `today`, `yesterday`, `<n> <unit> ago`, an RFC 3339 date-time with optional seconds and offset, a day whose year has four digits; `date` refusing an instant.
- src/email/search/filter/query.rs: `AfterInstant`, `BeforeInstant`, rendered back as RFC 3339.
- src/email/search/eval.rs, src/gmail/search.rs, src/jmap/backend.rs, src/imap/backend.rs (`InstantBand`, `outside_instants`), src/msgraph/search.rs (refused): each backend's instants.
- src/shared/envelope/search.rs: the `QUERY` help.
- Ten tests, `a_two_digit_year_is_refused` failing on e440b4e4.
- CHANGELOG (Added; Fixed for the year).

The Graph task ran no live check (no network in this batch): Graph refuses an instant until one shows KQL's `sent` takes a time of day.

## Capabilities moved

- [search](../spec/search.md): *One query language* modified, *Date values can be relative or instants* new.

[#528]: https://github.com/pimalaya/himalaya/issues/528
