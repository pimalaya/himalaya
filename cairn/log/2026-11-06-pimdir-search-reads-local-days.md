---
cairn: log
change: pimdir-search-reads-local-days
landed: 2026-11-06
---

# pimdir search reads local days

`pimdir search` (`--threads` included), `count` and `tag` state the local zone in `PimdirSearchClock::zone` (io-pimdir 5c18a02), so a `date:` year, month or day, `today` and `yesterday` are local ones, not UTC ones. Built on the move to io-pimdir 5c18a02 (pimdir-follows-storage-draft-09).

- src/pimdir/index.rs `local_clock`: jiff `TimeZone::try_system()` (which reads `TZ` first, then /etc/localtime or the platform's zone) and its `iana_name`. No name, as for a POSIX `TZ` string or a system zone jiff cannot name, or no zone keeps UTC (`None`) with one `warn!` saying to set `TZ`. The jiff Himalaya takes is calendula's (`std`, `tz-system`, `tzdb-bundle-always`), the same crate io-pimdir looks the name up in, so a name jiff gives io-pimdir knows.
- `PimdirClient::count_mail` and `tag_mail` take the clock, as `search_mail` takes it in its options; the commands pass `local_clock()`. `pimdir thread` takes no query and no clock.
- Test `a_day_is_read_in_the_zone_the_clock_states`: a message dated `Sat, 07 Nov 2026 00:30:00 +0100` (23:30 UTC on the 6th), a clock in Europe/Paris at 08:00 on the 7th: `date:2026-11-07` and `date:today` list it, `date:2026-11-06` does not, `count` gives 1 for `date:2026-11-07` and 0 for `date:yesterday`. Run with the clock's zone `None` it fails on the first assertion (`left: []`). The zone is stated at the client's boundary; the resolution itself reads the process's `TZ`, which the parallel tests do not mutate, so it has no test.
- The `pimdir search` help says its dates are read in the local zone.
- Mail carries no floating time, so the index's `floating_zone` is not read and no zone mismatch is printed, unlike calendula.

Out of scope: the shared `envelope list|search` date clauses on a pimdir account still read the UTC day (src/pimdir/backend.rs `PimdirPushdown`), the offset the store keeps.

Spec: search (pimdir searches through the store's index: the local zone paragraph and the scenario A day is the local day added).
