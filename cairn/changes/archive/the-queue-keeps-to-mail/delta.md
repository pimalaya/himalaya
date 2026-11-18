---
cairn: delta
change: the-queue-keeps-to-mail
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: The pimdir subcommand reads and retracts the queue
`queue cancel` SHALL refuse a row anchored on a collection that is not one of the account's mailboxes (another account's, or an address book or calendar another app of the store queued on), where it read a collection of another account; `queue show` SHALL read such a row as `unknown`.

## REMOVED Requirements

None.
