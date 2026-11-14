---
cairn: delta
change: tls-client-identity
---

## ADDED Requirements

### Requirement: A client certificate per backend
Every network backend's `tls` block MAY name a `client` table holding `cert` and `key`, the PEM certificate (optionally followed by its intermediates) and its private key, presented when the server asks for a client certificate. Both keys SHALL be required once the table is present.

## MODIFIED Requirements

### Requirement: Path keys expand as the configuration is read
Every path-valued key SHALL expand `~` and environment variables while the configuration is deserialized, not where it is read. This covers `maildir.root`, `mbox.root`, `mbox.inbox`, `pimdir.root`, the global and per-account `downloads-dir`, and the `tls.cert`, `tls.client.cert` and `tls.client.key` of every backend.

## REMOVED Requirements

None.
