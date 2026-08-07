---
cairn: log
change: pimdir-message-scoped-id
landed: 2026-08-02
---

# The pimdir public id is message-scoped (spec wording follow-up)

Follow-up to `pimdir-public-id`: io-pimdir's `seq` changed from per-collection to
message-scoped and store-global (`message-scoped-seq` there), so the same message
now shows the **same** id in every mailbox it is filed in. The backend needs no
code change — it already sets `Envelope.id = item.seq` and resolves it via
`get_item(collection, seq)` — but the spec wording said "per-collection". Corrected
the `backends` requirement to describe the id as a store-assigned integer, the same
across every mailbox a message is filed in.

Verified live: a message filed in INBOX and Archive shows id `1` in both, and
`message read 1 -m Archive` reads it.

Spec updated: `backends` (MODIFIED "pimdir shows a short public id" wording).
