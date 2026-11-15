---
cairn: change
id: a-utc-instant-ends-with-z
status: landed
created: 2026-11-15
---

# A UTC instant ends with `Z`

## Why

Task H10 of the family's cli-alignment-2026-11-15 plan (finding F14). Himalaya spelled a UTC instant two ways under `--json`: `Z` in `envelope list` and `search` (chrono's serde) and the `vacation` dates, `+00:00` in the read view's `headers.date` (mime-rs's `to_rfc3339`) and `imap fetch`'s `internalDate` (chrono's `to_rfc3339`). pimalaya-cli's cairn/spec/output.md says an instant a binary resolves is RFC 3339 in UTC with `Z`, a timestamp read from content keeping its offset: `Z` is that offset spelled once, and the one most of Himalaya already prints.

## What

The read view writes a zero offset `Z`, any other as found; `imap fetch` writes `internalDate` with `to_rfc3339_opts(SecondsFormat::AutoSi, true)`, chrono's serde spelling. What a provider or a store spells is kept. Breaking for a script matching `+00:00` in `imap fetch --internal-date` since v2: CHANGELOG and MIGRATION.md say so.
