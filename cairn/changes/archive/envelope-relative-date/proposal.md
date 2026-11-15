---
cairn: change
id: envelope-relative-date
status: landed
created: 2026-09-30
---

# Render recent envelope dates relative to today

## Why

Most listed messages are from today or yesterday, so a full date and offset spend width on what the reader already knows ([#510]). Alpine shows the time for today, `yesterday`, then the weekday.

## What

A new `envelope.list.datetime-relative` option, off by default, picks the DATE format per row from the message age, falling back to `datetime-fmt` past a week. The table only; JSON keeps the exact date.

[#510]: https://github.com/pimalaya/himalaya/issues/510
