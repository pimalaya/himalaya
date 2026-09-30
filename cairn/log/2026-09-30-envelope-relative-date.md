---
cairn: log
change: envelope-relative-date
landed: 2026-09-30
---

# Render recent envelope dates relative to today

Requested in [#510]: a listing is mostly today's and yesterday's messages, so a full date carries little.

## What landed

**`envelope.list.datetime-relative`**, global or per account, off by default. When on, the DATE column of `envelope list` and `envelope search` shows the time for today, `yesterday`, the weekday up to six days back, then `datetime-fmt`. It always uses the local timezone, since "today" only means something there, so `datetime-local-tz` has no effect on relative rows. Labels are English: chrono prints `%A` without locales. `--json` is unchanged.

## Capabilities moved

- config: *Envelope dates MAY render relative to today* is added

## Verification

Unit tests render a fixed today against each age bucket and a missing date, and check the absolute path is untouched when the option is off.

[#510]: https://github.com/pimalaya/himalaya/issues/510
