---
cairn: delta
change: pimdir-status-reports-the-store
---

# Delta

## ADDED Requirements

### Requirement: pimdir status reports the store

As folded into cairn/spec/backends.md: `himalaya pimdir status` prints `{account, accounts, mailboxes, parked, refused}`, one `{id, name, total, hydrated, queued}` per mailbox by id, each parked row `{row, handle, collection, kind, error}`, each refusal as `pimdir queue list` prints it; the text through `Listing`, a block per kind.

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
