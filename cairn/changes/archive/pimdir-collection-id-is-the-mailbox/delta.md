---
cairn: delta
change: pimdir-collection-id-is-the-mailbox
---

## REMOVED Requirements

### Requirement: pimdir names a mailbox the way its server does

## ADDED Requirements

### Requirement: A pimdir mailbox is its collection id
The pimdir backend SHALL show and accept a mailbox as the store's collection id, verbatim: the collection `imap/INBOX` is the mailbox `imap/INBOX`. It SHALL NOT derive, strip or accept a shortened spelling, and no configuration SHALL offer one.

The id is opaque to the store, which neither parses nor validates it (pimdir SPEC 9.2) and models hierarchy through `parent` rather than through a separator. Any shortening is therefore a guess at the producer's convention, and one that makes a single mailbox answer to two spellings. This is the JMAP backend's shape, whose ids are opaque server strings, and `[mailbox.alias]` is the shortcut for both.

`Mailbox.name` SHALL carry the collection row's own name rather than one derived from the id.

A mailbox matching no collection of the account SHALL be refused naming the ones it holds. It SHALL NOT be passed to the store unresolved, which reads as a mailbox that exists and is empty.

#### Scenario: A mailbox is addressed by its collection id
- GIVEN a pimdir account over a store whose mail collections are keyed `imap/<name>`
- WHEN the mailboxes are listed and one is addressed as `imap/INBOX`
- THEN both columns read `imap/INBOX` and the command targets that collection

#### Scenario: A shortened name is not a mailbox
- GIVEN the same account
- WHEN a command addresses `INBOX`
- THEN it is refused naming the collections the account holds
