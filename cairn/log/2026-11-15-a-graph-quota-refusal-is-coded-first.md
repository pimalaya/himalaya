---
cairn: log
change: a-graph-quota-refusal-is-coded-first
landed: 2026-11-15
---

# A Graph quota refusal is coded first

Task H2 of the family's cli-alignment-2026-11-15 plan (finding F28). `part_error` in src/msgraph/backend.rs matched `(403, _)` before `(_, "ErrorQuotaExceeded")`, so a quota refusal Graph sends as 403 was coded `permission-denied`, and a 507 without that code was not coded at all. Cardamum, Calendula, Carillon and Neverest read the quota first (cardamum/src/msgraph/client.rs `refusal_code`).

## What landed

- src/msgraph/backend.rs `part_error`: `ErrorQuotaExceeded` or a 507 is `quota-exceeded` whatever the status, then 429 `rate-limited`, then 403 `permission-denied`, the order of the four siblings. It is Himalaya's one Graph refusal mapping: the id lookups and the writes go through it, a listing fails whole, and `error::coded` codes account errors only.
- `a_quota_refusal_is_coded_whatever_its_status`: a `$batch` answer deserialized as io-msgraph's `MsgraphBatchResponses`, as `batch_all` reads it, then folded by `written`. It fails on the old order (`permission-denied` for the 403, `null` for the 507).

## Capabilities moved

- [backends](../spec/backends.md): the Microsoft Graph item codes, the quota read first.
- [commands](../spec/commands.md): the `permission-denied` and `quota-exceeded` rows of the code table.
