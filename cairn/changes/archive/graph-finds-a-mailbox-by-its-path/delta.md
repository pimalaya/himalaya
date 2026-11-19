---
cairn: delta
change: graph-finds-a-mailbox-by-its-path
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Graph folders are named by path
A mailbox value SHALL be resolved without listing the whole tree: a value shaped like a folder id first looked up with `GET /mailFolders/{value}`; else the folders walked only into a folder whose path followed by `/` begins the value, the folder whose path is the value taken, several refused naming their ids; else a role name looked up as the well-known folder carrying it, and a well-known name as itself. The resolved id SHALL be kept for the client's life and forgotten once the client creates, renames or deletes a folder. Without a `mailbox.alias.trash`, the trash of `message delete` SHALL be the well-known `deleteditems` folder, asked alone.

### Requirement: A Graph read is scoped to the mailbox given
The mailbox SHALL be resolved ("Graph folders are named by path") only when it is not the message's folder id.

## REMOVED Requirements
