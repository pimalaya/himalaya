---
cairn: tasks
change: a-queued-message-has-a-null-id
---

- [x] Decide the shape: the queue view's own envelope type with an optional id
- [x] `PimdirQueuedEnvelope` built by `queued_from_action`, `id` `None`
- [x] Test: a queued creation serialises with `"id": null` (failing first on `"id": ""`)
- [x] Fold into spec/backends.md, log, CHANGELOG
