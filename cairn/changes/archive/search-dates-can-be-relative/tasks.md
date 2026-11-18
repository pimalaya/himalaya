---
cairn: tasks
change: search-dates-can-be-relative
---

# Tasks

- [x] ~~Check whether Graph's KQL `sent>` and `sent<` accept a time of day, against the Microsoft test account~~ Dropped for this landing: the batch of 2026-11-18 ran without network nor accounts, so Graph refuses an instant with `search-unsupported` until a live run shows KQL takes a time of day.
- [x] src/email/search/filter: a date value type (day or instant); the parser reads the five forms with a now given by the caller (`query_at`); `date` refuses an instant; the keywords `ago`, `today`, `yesterday`; a year written with four digits, `15-11-26` refused where it read as the year 15.
- [x] Local zone as chrono's `Local` reads it (`TZ`, then the system's), UTC with one warning when neither names one.
- [x] Local evaluator: instants compared to the second (`AfterInstant`, `BeforeInstant`).
- [x] Gmail and JMAP: instants in `after:`/`before:` (Unix seconds) and in the UTCDate filters, the JMAP re-check on instants.
- [x] IMAP: the band of days around the instant searched with the query, the band's messages fetched and kept by date before the page is cut; `search-unsupported` under `or` and `not`.
- [x] Graph: refused with `search-unsupported`, per the first task.
- [x] The `QUERY` help and the long help.
- [x] Tests: `a_two_digit_year_is_refused` (failing on e440b4e4), `a_relative_date_counts_back_from_now`, `a_date_time_is_an_instant`, `a_date_value_of_no_known_shape_is_refused`, `an_instant_compares_to_the_second`, `an_instant_is_sent_in_seconds`, `an_instant_bounds_the_received_at_and_rechecks_the_sent_at`, `an_instant_is_searched_by_its_band_on_the_root_chain_alone`, `an_instant_keeps_the_messages_on_its_side` (loopback IMAP), `an_instant_is_refused` (Graph).
- [x] CHANGELOG; fold the delta into cairn/spec/search.md; log; status landed.
