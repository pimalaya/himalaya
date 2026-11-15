---
cairn: delta
change: a-top-mailbox-prints-a-null-parent
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir creates a mailbox by queueing an intent
`pimdir mailbox create <NAME> [--parent <MAILBOX>]` SHALL enqueue one `collection-create` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text, "parent": collection?}`, and print `{"row": i64, "name": text, "parent": collection | null, "source": id}`, `parent` `null` at the top, never left out. It SHALL NOT create a collection in the store: the mailbox arrives with the sync that performs the intent.

#### Scenario: A mailbox at the top
- GIVEN a pimdir account whose `gmail` source declares `collection.create`
- WHEN `himalaya --json pimdir mailbox create Projets` runs
- THEN it prints `"parent":null` beside `row`, `name` and `source`

## REMOVED Requirements

None.
