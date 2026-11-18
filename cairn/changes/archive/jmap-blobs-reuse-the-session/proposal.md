---
cairn: change
id: jmap-blobs-reuse-the-session
status: landed
created: 2026-11-18
---

# JMAP blobs reuse the session

## Why

Finding HIM-13 of the family's cli-family-analysis-2026-11-15 plan, its JMAP half: each blob download on a host other than the API's (Fastmail's case) built a new `SecretResolver`, ran the credential command again and opened a new TLS connection: one `message read` spawned the command twice, against config.md's "spawned once per command run". `jmap email import` did the same for its upload.

## What

The client keeps the `Authorization` value and the connect options it resolved with the session, and one auxiliary connection per foreign authority, reused for the next blob there and opened again once when it fails. The Graph half (each one-shot command walking the folder tree) is left to another change.
