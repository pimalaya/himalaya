---
cairn: delta
change: follows-storage-draft-17
---

# Delta

## ADDED Requirements

### Requirement: A reserved form is refused (search.md)
A text or a value beginning with `~`, one of two characters or more beginning with `/`, and a value beginning with `{` (pimdir SEARCH §8.3) SHALL refuse the whole query, its §8.4 code `reserved-form`: `search-invalid` on `envelope search` whatever the backend, `query-refused` on `pimdir search`, `count` and `tag`. Quoted, the characters are searched.

#### Scenario: A tilde word
- WHEN `himalaya --json envelope search '~draft'` runs on a Maildir account
- THEN it exits 1 coded `search-invalid`, the message naming `reserved-form` at `0..6`, and `envelope search '"~draft"'` lists the messages holding the word

### Requirement: Labels and properties are read from the store (search.md)
`pimdir search`, `count` and `tag` SHALL answer `label:` and `property:<key>=<value>` as io-pimdir answers them, from the store's item properties (pimdir STORAGE §4.3), never from the index. `envelope search` SHALL refuse both with `search-unsupported`, as a field outside mail. Himalaya stages no `set-property`.

### Requirement: A set-property row is listed (backends.md)
`pimdir queue list`, `queue show` and `queue cancel` SHALL take a `set-property` row another program queued on a mailbox of the account as any row: `kind` `set-property`, `itemId` the message it acts on, `subject` the stored message's, `linkId` `null`.

#### Scenario: A label queued by another program
- GIVEN a `set-property` row adding the label `todo` to message 7 of `imap/INBOX`
- WHEN `himalaya --json pimdir queue list -m imap/INBOX` runs
- THEN the row prints `"kind":"set-property"`, `"itemId":"7"`, `"linkId":null` and the subject of message 7

## MODIFIED Requirements

### Requirement: pimdir searches through the store's index (search.md)
The query is search-draft-17 §8. Text terms SHALL read the index as the sync engine last refreshed it; flag terms (`is:read`, `is:unread`, `is:flagged`, `is:answered`, `is:draft`, `tag:` naming a system flag) the store's flags as the sync engine last wrote them, even while the index lags (pimdir SEARCH §8.2). A queued write is still neither listed nor folded in.

### Requirement: One query language (search.md)
The refused fields gain `label:` and `property:`.

### Requirement: A pimdir store failure carries its code (backends.md)
Himalaya reads storage-draft-17. Every store an earlier build wrote, storage-draft-16 and earlier (all a development build of v3 wrote, storage-draft-15), is `store-version`, the earlier, in `PimdirFormatRefusal`'s wording.

### Requirement: A missing or outdated pimdir index is a coded error (search.md)
An index of search-draft-16 or earlier is `index-version` until the sync engine rebuilds it.

## REMOVED Requirements

None.
