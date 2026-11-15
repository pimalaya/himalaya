---
cairn: change
id: message-parse
status: landed
created: 2026-10-06
---

# `message parse`: the read view of a message without an account

## Why

The `message read` view (`message-read-view`) is reached through an account and a backend. A client often holds the raw message already: the draft it reopens, the message it forwards, the bytes it fetched once with `--raw`. Reading those again through the backend costs a round trip per part, and a client's tests, which start from raw messages, need a backend to see the view at all.

## What

- `message parse [EML]` reads a raw RFC 5322 message from a file path, or from stdin with `-` or when omitted, and prints the same output as `message read`: the designed view under `--json`, the same text rendering otherwise. It resolves no account and reads no configuration.
- `message parse --part <PART-ID>` writes that part's bytes (transfer encoding undone, as `attachment download --stdout`) to stdout instead; under `--json` it prints `{id, mime, filename, size, data}`, `data` in base64. Part ids are those of the view.
- The bounds and the `message-too-complex` code apply as on `message read`. A source holding nothing but whitespace is refused.
