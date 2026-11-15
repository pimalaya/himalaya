---
cairn: change
id: a-duplicate-mailbox-names-its-noun
status: landed
created: 2026-11-15
---

# A duplicate mailbox names its noun

## Why

Ruling D17 of the family's cli-alignment-2026-11-15 plan (report X3, row M4), now pimalaya-cli's cairn/spec/error.md (*A code names the domain's noun*, scenario *A collection created twice*): a create naming a collection that already exists fails everywhere with the collection noun's duplicate code. Himalaya answered three ways: a pimdir account failed `item-duplicate`, the item code; a Maildir or mbox account failed uncoded; IMAP, JMAP, Gmail and Graph relayed the server's refusal uncoded.

## What

- A new code, `mailbox-duplicate`: a create naming a mailbox that exists fails with it, exiting 1, nothing created nor staged. `item-duplicate` codes a message alone.
- Where each backend can tell: a Maildir folder already at the path (a file in the way is no mailbox, refused uncoded); an mbox file already there (a directory refused uncoded); a pimdir mailbox the store lists or has queued; an IMAP `CREATE` refused `ALREADYEXISTS` (RFC 5530), or refused with no code while `LIST` shows the mailbox (iCloud answers `NO Mailbox already exists`); a JMAP `Mailbox/set` refused while a sibling holds the name (RFC 8621 forbids two siblings sharing one), the refusal coded no other way; a Gmail label create answered 409; a Graph folder Graph answers 409 `ErrorFolderExists` at the top, or one of that name under the parent, looked up among the parent's children before the folder is created at the top, so no stray folder is left there.
- Every backend of Himalaya keys a mailbox by a name it can compare, so none lets two mailboxes share a name: no backend is left out.
- The unified `mailbox create`, `pimdir mailbox create` and the local stores' raw `maildir create` and `mbox create` code it; the protocol namespaces' own creates (`imap mailbox create`, `jmap mailbox create`, `gmail labels create`, `msgraph mail-folders create`) keep the server's refusal, as *A protocol refusal is a result* has it.
