---
cairn: delta
change: a-pimdir-file-names-its-part
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir lists the account's attachments
`pimdir attachment list` SHALL list the files the account's messages attach, read from the stand-ins of the account's attachments collection (pimdir STORAGE §14.3) through io-pimdir's `list_attachments_by_account`, newest message first, opening only the bodies of the page's messages, each once, to number their parts. It SHALL print `{"attachments": [...], "nextPage"}`, each file as `{"id", "name", "mediaType", "size", "part", "section", "messageId", "mailbox", "sender", "senderName", "date"}`: `id` the stand-in's public id; `name` its decoded name, `null` when the part names none; `mediaType` lowercased without parameters; `size` in octets once the transfer encoding is undone; `part` the id `attachment download` takes and `attachment list` and `message read` print, the position of the part standing at `section` in the message's body, `null` when the body is not local or holds no leaf part there; `section` its IMAP section in the message (RFC 3501 §6.4.5), which `message parse --section` takes; `messageId` and `mailbox` the public id and collection of one live placement of the message attaching it, as `envelope list` and `message read` take them; `sender` and `senderName` the message's sender; `date` its date, RFC 3339. Ids SHALL be strings.

#### Scenario: A file downloads by its part
- GIVEN a pimdir message whose body is `multipart/mixed` of a `multipart/alternative` text and a PDF, the PDF at section `2`
- WHEN `himalaya --json pimdir attachment list` runs
- THEN the PDF prints `"part":"5"` and `"section":"2"`, and `attachment download --stdout -m <mailbox> <messageId> 5` writes the PDF

### Requirement: Text output is plain text
`pimdir attachment list|search` print the file id, the size, the type, the part, the message id, the mailbox, the date, the sender's name or address, the filename; above, `Query:` for a search; paged by token.

## REMOVED Requirements

None.
