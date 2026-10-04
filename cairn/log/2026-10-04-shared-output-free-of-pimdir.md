---
cairn: log
change: shared-output-free-of-pimdir
landed: 2026-10-04
---

# Shared outputs drop the pimdir queue details

`envelope list` no longer carries a `queued` count, `message send`, `message add --send` and the composers no longer print a `queueId`, and the shared writes no longer end with notes or carry a `notes` array. Each field was filled by pimdir alone. The pimdir backend logs a queued send's row id at info level and a capability its source supports only in part as a warning; `himalaya pimdir queue list` shows what is queued. `EmailClient::send_message` returns only whether the backend filed the sent copy itself.

Capabilities moved: **backends** (shared outputs carry no backend details; queued creations and sends shown by `pimdir queue list` alone).
