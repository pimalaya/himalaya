---
cairn: log
change: io-pimdir-empty-mail-date
landed: 2026-10-07
---

# io-pimdir pinned at f9b13f8, an empty mail date stored as no date

io-pimdir moves from `35a1c3f` to `f9b13f8`, the fix for an empty mail date. That commit folds every summary the hub stores through `PimdirSummary::stored`, which reads an empty mail date as none (STORAGE Annex A.1), so a round opened and closed in one run and a resumed round agree on an undated message. Himalaya's code is unchanged, as the fix lives in the hub and changes no API it uses. Tests run with the default features and with `--no-default-features --features pimdir,rustls-ring`, as MOA builds it.

Capabilities moved: none.
