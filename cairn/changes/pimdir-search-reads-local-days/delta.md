---
cairn: delta
change: pimdir-search-reads-local-days
---

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir searches through the store's index (search, a paragraph and a scenario added)
A `date:` term's years, months and days, `today` and `yesterday` SHALL be read in the local zone (SEARCH §8.1), in `pimdir search`, `count` and `tag` alike: `TZ`, then the system's zone, by its IANA name. A zone with no IANA name, a POSIX `TZ` string or a system zone that cannot be named, or no zone at all, SHALL leave them in UTC with one warning.

#### Scenario: A day is the local day
- GIVEN `TZ=Europe/Paris` and an indexed message received at 00:30 on 2026-11-07 in that zone
- WHEN `himalaya pimdir search date:2026-11-07` runs
- THEN the message is listed, and `date:2026-11-06` does not list it

## REMOVED Requirements

None.
