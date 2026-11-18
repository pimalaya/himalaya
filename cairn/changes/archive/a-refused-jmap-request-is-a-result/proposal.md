---
cairn: change
id: a-refused-jmap-request-is-a-result
status: landed
created: 2026-11-18
---

# A refused JMAP request is a result

## Why

The live run of 2026-11-18 (K4): `jmap request` with an unknown capability printed the failure object and exited 1, where *A protocol refusal is a result* asks for the refusal object and exit 3. io-jmap's `JmapClientStdError` wraps the `JmapSendError` transparently, so the error chain lists the wrapper and then the send error's source, never the send error, and src/jmap/error.rs never read it.

## What

- src/jmap/error.rs reads the send, session and blob errors io-jmap's client error wraps, for the refusal and for its code.
- A scenario pins the request refused whole. No behaviour beyond the spec changes.
