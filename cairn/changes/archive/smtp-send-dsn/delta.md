---
cairn: change
change: smtp-send-dsn
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Sending transport
The protocol-level `smtp send` SHALL additionally request delivery status notifications (RFC 3461) through `--notify`, `--ret` and `--envid`, failing before `MAIL FROM` when the server does not announce `DSN`. The shared `message send` requests none.

## REMOVED Requirements
