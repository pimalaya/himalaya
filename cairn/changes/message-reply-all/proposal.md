---
cairn: change
id: message-reply-all
status: landed
created: 2026-10-14
---

# `message reply --all`

## Why

`message reply` derives its recipients from `Reply-To`, or `From` when there is none, and has no way to answer everyone: the other `To` and `Cc` recipients of the source have to be read from `message read --json` and passed back by hand with `--to` and `--cc` ([#779](https://github.com/pimalaya/himalaya/issues/779)).

## What

- `--all` (`-A`) on `message reply`.
- `To` stays as it is today. `Cc` is the `To` and `Cc` of the source, without the account's `email` nor anyone already in `To`, deduplicated ignoring case, display names kept.
- An explicit `--to` or `--cc` wins over the list it would derive; the `To` that stands, explicit or derived, is the one `Cc` is deduplicated against.
- `message forward` is unchanged.

Left out: only the account's `email` is filtered, not its aliases or the `--from` address. `Mail-Followup-To` and `Mail-Reply-To` are not read.
