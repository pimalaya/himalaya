---
cairn: delta
change: a-listed-mailbox-is-not-created-again
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir creates a mailbox by queueing an intent
The row SHALL be anchored on `parent`, which must be a mailbox of the account, else on the account's first mailbox by id, and queued through io-pimdir's `enqueue_collection_create`. The performer SHALL be named as for a send (capability `collection.create`): several candidates and no recorded choice are refused, naming them. A store whose sources declare no capabilities SHALL be refused, its owner predating the intent. An empty name, or one holding a control character, SHALL be refused before anything is staged. A mailbox the store lists already SHALL be refused with `item-duplicate`, naming it, before anything is queued, as "Unified mailbox writes" refuses creating an existing mailbox: one the performer's source syncs under NAME at the top, or under the parent's name, a hierarchy delimiter (`/` or `.`, the store recording none) and NAME, names compared exactly; so SHALL a creation of NAME under that parent the same source has queued already. A name the store does not list yet is the server's to refuse: the row parks, and `pimdir queue show` says why.

#### Scenario: A mailbox the store lists
- GIVEN a pimdir account whose `imap` source declares `collection.create` and syncs `imap/Projets`, named `Projets`
- WHEN `himalaya mailbox create Projets` or `himalaya pimdir mailbox create Projets` runs
- THEN the command fails with the code `item-duplicate`, naming `imap/Projets`, and nothing is queued

### Requirement: A JSON error carries a stable code
The code table's `item-duplicate` row reads: a pimdir mailbox holds the message already, or has its creation queued, or a pimdir store lists the mailbox a creation names, or has that creation queued; nothing is staged.

## REMOVED Requirements

None.
