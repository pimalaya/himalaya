---
cairn: delta
change: a-pimdir-role-names-its-mailbox
---

## MODIFIED Requirements

### Requirement: A pimdir mailbox is its collection id
The pimdir backend SHALL show and accept a mailbox as the store's collection id, verbatim: the collection `imap/INBOX` is the mailbox `imap/INBOX`. It SHALL NOT derive, strip or accept a shortened spelling, and no configuration SHALL offer one.

The id is opaque to the store, which neither parses nor validates it (pimdir STORAGE §9.2) and models hierarchy through `parent` rather than through a separator. Any shortening is therefore a guess at the producer's convention, and one that makes a single mailbox answer to two spellings. This is the JMAP backend's shape, whose ids are opaque server strings, and `[mailbox.alias]` is the shortcut for both.

`Mailbox.name` SHALL carry the collection row's own name rather than one derived from the id.

A mailbox is a collection declared `message/rfc822`, matched on the bare media type. A collection declaring no kind is not a mailbox: the store cannot summarise it, and the draft that left kinds undeclared is refused on open rather than read.

A value matching no collection id of the account SHALL be taken as a role name when it is one (`inbox`, `sent`, `drafts`, `trash`, ...), addressing the account's mailbox whose recorded role (`collections.role`) it is; several mailboxes carrying the role SHALL be refused naming them. A collection's name SHALL NOT address it: two sources may hold mailboxes of one name. A value matching neither SHALL be refused naming the mailboxes the account holds. It SHALL NOT be passed to the store unresolved, which reads as a mailbox that exists and is empty.

#### Scenario: The inbox by its role
- GIVEN a pimdir account whose store records the role `inbox` on `imap/INBOX`
- WHEN `envelope list` runs without `-m`
- THEN the envelopes of `imap/INBOX` are listed

#### Scenario: The trash by its role
- GIVEN a pimdir account with no `mailbox.alias.trash` and a mailbox recorded with the role `trash`
- WHEN `message delete` runs on a message of another mailbox
- THEN the message is staged as a move to that mailbox

### Requirement: Mailbox role
A shared mailbox SHALL carry an optional role (inbox, all, archive, drafts, flagged, important, junk, sent, subscribed, trash, or a verbatim unknown one), shown by `mailbox list` in its table and JSON output. JMAP reads it from the mailbox `role`, Gmail from its fixed system-label ids, Microsoft Graph from its well-known folder names resolved to folder ids in one `$batch` request, pimdir from the role its sync engine recorded from the server (`collections.role`), and IMAP marks only `INBOX`, as mbox marks its spool when one is configured. Maildir and m2dir have no native role. On JMAP, Gmail, Graph and pimdir a role name addresses the mailbox carrying it. A `mailbox.alias.<role>` entry overrides the native role of the mailbox it names, and `message delete` takes its trash from that alias before the backend's trash role.
