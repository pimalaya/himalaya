---
cairn: delta
change: pimdir-producer-reader
---

## ADDED Requirements

### Requirement: pimdir names a mailbox the way its server does
A hub collection is keyed `<namespace>/<name>`, so the pimdir backend SHALL show
and accept the name without the namespace: the collection `imap/INBOX` is the
mailbox `INBOX`. The namespace SHALL be derived when every mail collection of the
account shares one prefix, which a single-source account always does;
`pimdir.namespace` overrides it, and a store whose mail collections span two
namespaces keeps whole ids as names rather than collapsing two mailboxes onto one.

A user-typed name SHALL resolve against the account's mail collections, a full
collection id still being taken as itself. A name matching none, or several, SHALL
be refused naming what the account holds. It SHALL NOT be passed to the store
unresolved, which reads as a mailbox that exists and is empty.

#### Scenario: The configured inbox alias resolves
- GIVEN a store whose collections are keyed under the namespace `imap`
- WHEN a command runs with `-m INBOX`, or with none and `mailbox.alias.inbox = "INBOX"`
- THEN it lists `imap/INBOX`

#### Scenario: An unknown mailbox says what there is
- GIVEN the same store
- WHEN a command runs with `-m Nope`
- THEN it fails naming the mailboxes the account holds, listing nothing

## MODIFIED Requirements

### Requirement: pimdir is a reader and a producer, never the owner
The pimdir backend SHALL treat the store as a possibly-partial cache owned by the
sync engine. `get_message` on an item whose body is not local (`level < Full`, no
stored object) SHALL report a clear "body not fetched" state (the cue to sync),
not a data-loss error; the item still lists.

Reads SHALL open the store read-only, which takes no lock (pimdir SPEC §8), so a
sync in flight neither blocks Himalaya nor is blocked by it.

A write SHALL be staged as a queued `PimdirAction` through a producer handle
(`store_flags`→`SetFlags`, `add_message`→`Add`, `copy_messages`→`Copy`,
`move_messages`→`Move`, `delete_messages`→`Remove`), addressed by the public
`seq`, for the store's owner to apply and push. The backend SHALL NOT write the
index, load a collection, or run the owner's object sweep: a sweep run beside a
sync destroys the bodies it has streamed but not yet attached, which SPEC §14
explicitly invites it to leave pending. A body an action references SHALL be
written to the blob store durably before the action is enqueued, the queue row
being what pins it.

`SetFlags` carries the whole replacement set, so applying it twice lands the same
state; a set the store reports as unknown contributes no markers rather than
staging an unknown one, which would erase what a sync knows. pimdir has no native
trash.

An added message SHALL derive its link id, summary and sort key through
`io_pimdir::conventions`, the one implementation of SPEC Annex A, and SHALL spell
the link id the way the store it writes to already does, so an added message
deduplicates against a synced copy rather than linking it a second time.

### Requirement: pimdir reads one account
The pimdir backend SHALL show the collections of one account (pimdir SPEC §9.2),
`pimdir.account` naming it. Unset, it is derived: a store holding one account, or
one ungrouped set, is read as that one, and a store holding several is refused
naming them rather than guessing one and showing the wrong mailbox set.

### Requirement: pimdir shows a short public id
The pimdir backend SHALL show and accept each message's public id (`items.seq`, a
small store-assigned integer, the same across every mailbox the message is filed
in) as its `Envelope.id`, not the internal `link_id`. It SHALL check the id
against the collection before reading a body or staging an action, and SHALL fail
clearly on a non-numeric or unknown one. `add_message` SHALL return the link id it
staged: a queued create has no `seq` yet, the store assigning one when its owner
applies the action.

## REMOVED Requirements

### Requirement: pimdir writes auto-source
A producer attributes no action to a source: the owner applies the queue as
itself. `pimdir.source` is gone, replaced by `pimdir.account`, which answers the
question a reader actually has.
