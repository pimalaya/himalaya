---
cairn: delta
change: the-page-token-is-long-only
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir lists the account's attachments
Its paging paragraph reads:

A page SHALL hold `--page-size` files (else `envelope.list.page-size`, else 25). `nextPage` SHALL be an opaque token that `--page-token`, long only (`-t` is `--to` across Himalaya), takes to start after the page's last file, and `null` exactly when no file follows. Pages SHALL follow one another by keyset on the message's sort key and the stand-in's id, so a write between two pages neither repeats nor skips a file already listed. A token this command did not print SHALL be refused. An account with no attachments collection, or a store without stand-ins, SHALL list none rather than fail.

## REMOVED Requirements

None.
