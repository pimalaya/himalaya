---
cairn: change
id: shared-output-free-of-pimdir
status: landed
created: 2026-10-04
---

# Keep pimdir details out of the shared outputs

## Why

The shared commands are a protocol-agnostic API, and pimdir is one backend among several. Yet their outputs grew pimdir-only fields: a `queued` count on `envelope list`, a `queueId` on `message send`, `message add --send` and the composers, and a `notes` array wrapping every write. Each was empty or absent on every other backend, so they described one backend's store rather than the operation.

## What

The shared outputs drop `queued`, `queueId` and `notes`. A queued send logs its row id at info level, a write a source supports only in part logs a warning, and `himalaya pimdir queue list` remains the place to see what is queued. `EmailClient::send_message` returns only whether the backend filed the sent copy itself, documented without naming a backend.
