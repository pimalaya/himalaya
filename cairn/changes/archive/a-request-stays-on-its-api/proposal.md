---
cairn: change
id: a-request-stays-on-its-api
status: landed
created: 2026-11-15
---

# A request stays on its API

## Why

The owner's call on the request escape hatches (2026-11-15): `gmail request` and `msgraph request` took a full URL as given and sent the account's bearer token to any host it named. The Graph page tokens already stay on Graph's origin (io-msgraph's `page_get`); Gmail's page tokens are opaque and JMAP's page numbers, so no other path follows a URL.

## What

- `gmail request` and `msgraph request` refuse a URL off their API's origin before sending (src/shared/raw_json.rs `api_url`).
