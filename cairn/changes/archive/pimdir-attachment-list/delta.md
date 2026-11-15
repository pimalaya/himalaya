---
cairn: change
change: pimdir-attachment-list
---

# Delta

## ADDED Requirements

### Requirement: pimdir lists the account's attachments
`pimdir attachment list` SHALL list the files the account's messages attach, read from the stand-ins of the account's attachments collection (pimdir STORAGE §14.3) through io-pimdir's `list_attachments_by_account`, newest message first, opening no body. It SHALL print `{"attachments": [...], "nextPage"}`, each file as `{"id", "name", "mediaType", "size", "section", "messageId", "mailbox", "sender", "senderName", "date"}`: `id` the stand-in's public id; `name` its decoded name, `null` when the part names none; `mediaType` lowercased without parameters; `size` in octets once the transfer encoding is undone; `section` its IMAP section in the message (RFC 3501 §6.4.5); `messageId` and `mailbox` the public id and collection of one live placement of the message attaching it, as `envelope list` and `message read` take them; `sender` and `senderName` the message's sender; `date` its date, RFC 3339. Ids SHALL be strings.

A page SHALL hold `--page-size` files (else `envelope.list.page-size`, else 25). `nextPage` SHALL be an opaque token that `--page-token` takes to start after the page's last file, and `null` exactly when no file follows. Pages SHALL follow one another by keyset on the message's sort key and the stand-in's id, so a write between two pages neither repeats nor skips a file already listed. A token this command did not print SHALL be refused. An account with no attachments collection, or a store without stand-ins, SHALL list none rather than fail.

`pimdir attachment search <WORDS>...` SHALL print the same page, keeping the files for which every word appears, ignoring case (Unicode lowercase; accents count), in the file name, the sender's address or the sender's name. It SHALL read the listing newest first and stop once the page and one more file are found. A query of no word SHALL be refused.

#### Scenario: A page of files and the next one
- GIVEN a pimdir account whose stored messages attach four files
- WHEN `himalaya --json pimdir attachment list -s 3` runs
- THEN it prints the three files of the newest messages and a `nextPage` token, and `pimdir attachment list -s 3 --page-token <TOKEN>` prints the fourth with `"nextPage":null`

#### Scenario: Invoices from one sender
- GIVEN a pimdir account where `billing@shop.test` sent `Facture-Octobre.pdf` and another sender `plage.jpg`
- WHEN `himalaya --json pimdir attachment search facture billing` runs
- THEN it prints `Facture-Octobre.pdf` only

## MODIFIED Requirements

### Requirement: A parsed message hands one part over
`message parse --part <PART-ID>` SHALL write that part's bytes, transfer encoding undone, to stdout, or under `--json` `{id, mime, filename, size, data}` with `data` in base64; the ids SHALL be those of the view. An unknown id or a container's SHALL be refused.

`message parse --section <SECTION>` SHALL do the same for the leaf part at that IMAP section (RFC 3501 §6.4.5): `1` for the body of a message that is not multipart, else each part's 1-based position among its siblings, joined with dots down the message's own multiparts. The JSON `id` SHALL stay the view's. A container's section, a part of an attached message, or a section naming nothing SHALL be refused; `--part` and `--section` SHALL NOT be given together.
