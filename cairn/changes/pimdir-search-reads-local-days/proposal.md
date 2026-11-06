---
cairn: change
id: pimdir-search-reads-local-days
status: landed
created: 2026-11-06
---

# pimdir search reads local days

## Why

`himalaya pimdir search`, `count` and `tag` read `date:` years, months and days, `today` and `yesterday` in UTC: io-pimdir up to 0c70144 cut every span at UTC midnight. In Paris `date:2026-11-07` misses a message received at 00:30 on 7 November (23:30 UTC on the 6th), and `today` turns over at 01:00. SEARCH §8.1 reads a span in the zone the client states, so Himalaya states the user's. calendula and neverest make the same move (calendula cairn/changes/pimdir-search-reads-local-days).

## What

- The clock of `pimdir search` (`--threads` included), `count` and `tag` states the local zone in `PimdirSearchClock::zone` (io-pimdir 5c18a02, change pimdir-follows-storage-draft-09): `TZ`, then the system's zone (jiff `TimeZone::try_system`, which reads `TZ` first), by its IANA name. A zone with no IANA name (a POSIX `TZ` string, a system zone jiff cannot name) or none at all keeps UTC, with one warning saying to set `TZ`.
- `pimdir thread` takes no query and states no clock.
- `PimdirClient::count_mail` and `tag_mail` take the clock, as `search_mail` takes it in its options, so a test states the zone at that boundary rather than mutating the process's `TZ`.
- Mail carries no floating time, so the index's `floating_zone` does not bear on a mail search: Himalaya names no zone mismatch, unlike calendula.
