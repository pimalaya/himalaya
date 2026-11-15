---
cairn: change
change: pimdir-mailbox-create
---

# Delta

## ADDED Requirements

### Requirement: pimdir creates a mailbox by queueing an intent
`pimdir mailbox create <NAME> [--parent <MAILBOX>]` SHALL enqueue one `collection-create` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text, "parent": collection?}`, and print `{"queueId": i64, "name": text, "parent"?: collection, "source": id}`. It SHALL NOT create a collection in the store: the mailbox arrives with the sync that performs the intent.

The row SHALL be anchored on `parent`, which must be a mailbox of the account, else on the account's first mailbox by id. The performer SHALL be named as for a send (capability `collection.create`): several candidates and no recorded choice are refused, naming them. A store whose sources declare no capabilities SHALL be refused, its owner predating the intent. An empty name, or one holding a control character, SHALL be refused before anything is staged. A name the server already holds is the server's to refuse: the row parks, and `pimdir queue show` says why.

#### Scenario: A folder is created through the sync engine
- GIVEN a pimdir account whose `imap` source declares `collection.create`
- WHEN `himalaya --json pimdir mailbox create Projets` runs
- THEN one `collection-create` row is queued with `{"v":1,"source":"imap","name":"Projets"}`, and the command prints its `queueId` and `"source":"imap"`
