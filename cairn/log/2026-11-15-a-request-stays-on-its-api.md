---
cairn: log
change: a-request-stays-on-its-api
landed: 2026-11-15
---

# A request stays on its API

The owner's call on the request escape hatches.

## What landed

- `gmail request` and `msgraph request` refuse a URL off the origin of their API before sending (src/shared/raw_json.rs `api_url`); the Graph page tokens already did, and no other path follows a URL.

## Capabilities moved

- [commands](../spec/commands.md): *The Gmail and Graph escape hatches are requests* modified.
