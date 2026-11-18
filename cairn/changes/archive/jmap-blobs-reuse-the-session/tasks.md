---
cairn: tasks
change: jmap-blobs-reuse-the-session
---

# Tasks

- [x] src/jmap/client.rs: `http_auth`, `options`, `auxiliary`.
- [x] src/jmap/email/import.rs: the upload through `auxiliary`.
- [x] Test `blob_downloads_reuse_the_credential_and_the_connection` (two loopback hosts, a counting credential command), failing on 002bf012 (three runs).
- [x] Fold into cairn/spec/provider-quirks.md; CHANGELOG.
