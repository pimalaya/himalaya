---
cairn: change
change: pimdir-mailbox-rename-delete
---

# Delta

## ADDED Requirements

### Requirement: pimdir renames or deletes a mailbox by queueing an intent
`pimdir mailbox rename <MAILBOX> <NAME>` SHALL enqueue one `collection-rename` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text}` and print `{"queueId": i64, "mailbox": collection, "name": text, "source": id}`. `pimdir mailbox delete <MAILBOX>` SHALL enqueue one `collection-delete` intent with `{"v": 1, "source": id}` and print `{"queueId": i64, "mailbox": collection, "source": id}`. Neither SHALL rename or delete a collection in the store: the change arrives with the sync that performs the intent.

The row SHALL be anchored on the mailbox, which must be a mailbox of the account, the payload never restating it. The performer SHALL be named as for a creation (capability `collection.rename` or `collection.delete`). A store whose sources declare no capabilities SHALL be refused. A rename name empty or holding a control character SHALL be refused before anything is staged. A name holding the server's separator, a sibling of the same name and a mailbox holding others are the performer's to refuse: the row parks.

## MODIFIED Requirements

### Requirement: pimdir writes name their queue row
A row the sync engine performed rather than applied, a message sent or a mailbox created, renamed or deleted, reads as `applied` without `seq`. A receipt follows a renamed mailbox to its new id, and leaves the store with a deleted one, whose row then reads `unknown`.

### Requirement: pimdir stages flags by their raw spelling
A change on `$Important` or `$Junk` SHALL be gated by `mail.flags.important` or `mail.flags.junk` from a source stating that row, and by `mail.flags.keywords` from one stating none (pimdir STORAGE Annex B.1).
