---
cairn: log
change: pimdir-queue-row
landed: 2026-10-04
---

# pimdir writes name their queue row

`himalaya pimdir message add` prints `{"queueId","messageId"}` and `himalaya pimdir message send` `{"queueId","messageId","copy","copyQueueId"?}`, staging exactly what the shared commands stage. `himalaya pimdir queue show <ROW>` prints `{"queueId","state","collection"?,"kind"?,"attempts"?,"error"?}`, the state `pending`, `parked` or `gone`. The shared outputs are unchanged.

Not landed: the applied `seq`. io-pimdir deletes a row once applied and keeps no record of the item it produced, so `gone` covers applied and cancelled alike.

Capabilities moved: **backends** (pimdir writes name their queue row).
