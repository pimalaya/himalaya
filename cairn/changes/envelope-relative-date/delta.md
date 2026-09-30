---
cairn: change
change: envelope-relative-date
---

# Delta

## ADDED Requirements

### Requirement: Envelope dates MAY render relative to today
`envelope.list.datetime-relative`, settable globally and per account, SHALL render the DATE column of `envelope list` and `envelope search` relative to today in the local timezone: the time of day (`%R`) for today, `yesterday` for the day before, the weekday (`%A`) for the five days before that, and `datetime-fmt` for anything older or dated in the future. It SHALL default to `false`, and SHALL NOT change the `--json` output, which keeps the exact date.
