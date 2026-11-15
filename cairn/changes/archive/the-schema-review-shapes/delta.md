---
cairn: delta
change: the-schema-review-shapes
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Composers default the From header
`messages compose`, `messages reply` and `messages forward` SHALL fill the `From` header from the resolved account when `--from` is not passed: `email` as the address, `display-name` as the name it carries. An explicit `--from` SHALL win whole, the configured name never being grafted onto an address the user spelled out. With neither, the header SHALL be omitted rather than guessed.

The name SHALL be handed to the MIME builder apart from the address, so that a name carrying a comma, a quote or a non-ASCII character is encoded by the builder rather than by a quoting rule of Himalaya's own.

The composers SHALL write the `Date` (the current time at the local offset) and the `Message-ID` themselves, mime-rs's builder writing no field it is not given: the id sits on the domain of the `From` address when it is a plain dot-atom, else on the machine's hostname, else on `localhost`, its left part unique on the machine.

The same composers SHALL end the body with the account's `signature`, introduced by its `signature-delim`, when neither `--signature` nor `--signature-file` is passed. `--signature` SHALL win, and `--signature-file` SHALL win too, the configured signature standing down rather than shadowing the file the flag names.

With `--json` and neither `--save` nor `--send`, the same composers SHALL print the decoded `from`, `to`, `cc`, `bcc`, `subject`, `body` and `attachments` instead of the raw bytes, each address as the envelopes print one, `{name, email}` with `name` decoded or `null`, `from` the first `From:` mailbox or `null`, `to`, `cc` and `bcc` lists of them, groups flattened, and `subject` `null` without one, the body without any signature and with LF line breaks, `attachments` listing each attached file as `{filename, mime, size}` (the template carries no bytes: handing it back means passing the files again through `--attach`), so an editor can lay the message out and hand it back through the flags, sending appending the signature once. `--posting-style none` SHALL send the written body alone, the source left unquoted, for a writer who already laid the quote out.

The text body a composer writes SHALL end with a line break, so a printed single-part message ends on one, and a quoted empty line SHALL be `>` with no trailing blank. `--no-save` SHALL be refused without `--send`. An attached message (`message/rfc822`, from `--attach` or a forward's source) SHALL be written under 7bit, 8bit or binary only (RFC 2046 §5.2.1), its line breaks CRLF, or as the nested message it holds with its own parts encoded when 8-bit content cannot go as is.

### Requirement: The auto-reply is in the unified API
`vacation get` SHALL print the account's auto-reply as `{enabled, start, end, subject, text, html}`, `start` and `end` the window it answers in, RFC 3339 in UTC, `null` for no bound (`Start:` and `End:` in text), and the default one, off with nothing set, when the backend holds none. `vacation set` SHALL change only what it is given (`--enable` or `--disable`, `--from`, `--to` as RFC 3339 or a `YYYY-MM-DD` read as midnight UTC, `--subject`, `--text`, `--html`, each cleared by its `--no-from`, `--no-to`, `--no-subject`, `--no-text`, `--no-html`, which conflicts with its setter) and print the auto-reply as it then stands; given none of these options, it SHALL fail as a usage error before connecting. JMAP serves it when its session advertises `urn:ietf:params:jmap:vacationresponse` (RFC 8621 section 8), Gmail through `settings.getVacation` and `updateVacation`, its dates epoch milliseconds, the settings read first so the Gmail-only ones survive.

### Requirement: A write prints what it acted on
Every write SHALL print what it acted on, never a sentence alone. A unified write SHALL print its receipt with `queued`: `message copy` and `message move` `{count, queued}`, `message add` `{id, sent, queued}`, `id` the added message's and `sent` whether `--send` sent it too, `message send` and a composer's save or send `{id, saved, sent, queued}`, `id` being the saved copy's, `null` when the message was not saved, the save is queued, or the backend returned none. `message delete` SHALL print the receipt of a batch delete, `{action, count, queued, errors}`: it takes several ids, so it counts the messages it acted on as `message copy` and `message move` do, where a delete of one item names it (`{id, queued}`), and `action` says what became of them, `moved-to-trash`, `deleted` from the trash, or `flagged` `\Deleted` with an expunge owing. A composer neither saving nor sending SHALL print the template under `--json` and the message bytes otherwise. A protocol write with no payload of its own (Gmail, Microsoft Graph, JMAP, maildir, mbox, ManageSieve) SHALL print `{id}` naming the item it acted on as it is afterwards, or `{ids}` for several, in the order acted on; its text form SHALL keep the sentence. `jmap email import` SHALL print `{id, blobId}`, `id` `null` with `--upload-only`; `msgraph messages send` `{id: null}`, `sendMail` answering without one; `smtp send` the envelope it sent, `{mailFrom, rcptTo}`.

#### Scenario: A Gmail label is created
- GIVEN a Gmail account
- WHEN `himalaya --json gmail labels create Projects` runs
- THEN it prints `{"id":"Label_12"}`, the id Gmail gave the label

### Requirement: Absent is null
An output type of Himalaya's own SHALL print every key, `null` when the value is absent, so its schema marks every key required. A message without a `Subject:` SHALL print `subject` as `null`, not as an empty string, in every envelope, search hit, thread entry (`imap thread`, `pimdir thread`), queued row and `pimdir tag` row. A provider payload printed as received SHALL keep its own omissions.

### Requirement: A local store flag prints its meaning beside its spelling
Every flag Himalaya prints SHALL be the one `Flag` noun, `{raw, iana}`, `raw` the spelling the store keeps and `iana` the IANA keyword it stands for, `null` for one none registers: an envelope's `flags`, `pimdir flag list` (its `flags` `null` when the sync engine never read them), the flags of a `pimdir tag` row, and the rows of `maildir flag list` and `mbox flag list`, `raw` there being the store's letter and `iana` `null` for mbox's `O`, which has none; mbox rows SHALL also name the `header` carrying the letter.

### Requirement: pimdir lists the account's attachments
`pimdir attachment list` SHALL list the files the account's messages attach, read from the stand-ins of the account's attachments collection (pimdir STORAGE §14.3) through io-pimdir's `list_attachments_by_account`, newest message first, opening only the bodies of the page's messages, each once, to number their parts. It SHALL print `{"attachments": [...], "nextPage"}`, each file as `{"id", "filename", "mime", "size", "part", "section", "message", "mailbox", "from", "date"}`, the keys `attachment list` spells alike: `id` the stand-in's public id; `filename` its decoded name, `null` when the part names none; `mime` its MIME type, lowercased without parameters; `size` in octets once the transfer encoding is undone; `part` the id `attachment download` takes and `attachment list` and `message read` print, the position of the part standing at `section` in the message's body, `null` when the body is not local or holds no leaf part there; `section` its IMAP section in the message (RFC 3501 §6.4.5), which `message parse --section` takes; `message` and `mailbox` the public id and collection of one live placement of the message attaching it, as `envelope list` and `message read` take them; `from` the message's sender as an envelope address, `{name, email}`, `null` when the store records no address; `date` its date, RFC 3339. Ids SHALL be strings.

A page SHALL hold `--page-size` files (else `envelope.list.page-size`, else 25). `nextPage` SHALL be an opaque token that `--page-token`, long only (`-t` is `--to` across Himalaya), takes to start after the page's last file, and `null` exactly when no file follows. Pages SHALL follow one another by keyset on the message's sort key and the stand-in's id, so a write between two pages neither repeats nor skips a file already listed. A token this command did not print SHALL be refused. An account with no attachments collection, or a store without stand-ins, SHALL list none rather than fail.

`pimdir attachment search <WORDS>...` SHALL print the same page, keeping the files for which every word appears, ignoring case (Unicode lowercase; accents count), in the file name, the sender's address or the sender's name. It SHALL read the listing newest first and stop once the page and one more file are found. A query of no word SHALL be refused.

#### Scenario: A page of files and the next one
- GIVEN a pimdir account whose stored messages attach four files
- WHEN `himalaya --json pimdir attachment list -s 3` runs
- THEN it prints the three files of the newest messages and a `nextPage` token, and `pimdir attachment list -s 3 --page-token <TOKEN>` prints the fourth with `"nextPage":null`

#### Scenario: A file downloads by its part
- GIVEN a pimdir message whose body is `multipart/mixed` of a `multipart/alternative` text and a PDF, the PDF at section `2`
- WHEN `himalaya --json pimdir attachment list` runs
- THEN the PDF prints `"part":"5"` and `"section":"2"`, and `attachment download --stdout -m <mailbox> <messageId> 5` writes the PDF

#### Scenario: Invoices from one sender
- GIVEN a pimdir account where `billing@shop.test` sent `Facture-Octobre.pdf` and another sender `plage.jpg`
- WHEN `himalaya --json pimdir attachment search facture billing` runs
- THEN it prints `Facture-Octobre.pdf` only

### Requirement: pimdir stages flags by their raw spelling
`himalaya pimdir flag list|add|set|remove` SHALL read and stage the flags of a pimdir message by their raw spelling, verbatim. A value naming an IANA flag SHALL be accepted only in its canonical spelling, compared without case and staged in the canonical case, a bare `seen` or `$seen` being refused naming `\Seen`; `\Deleted` and `\Recent` SHALL be refused. A backslash names a system flag, so one in front of any other word SHALL be refused, as SHALL a character IMAP forbids in a flag (`(`, `)`, `{`, `%`, `*`, `"`, `]`, `\`). A value the store cannot hold (pimdir STORAGE §13), empty or holding whitespace, a control character or a character outside ASCII, SHALL be refused with the code `flag-invalid`, the code io-pimdir's enqueue gives a flag set holding one, and nothing staged. Any other value is a keyword.

`add`, `set` and `remove` SHALL fail when no message is named or none of the named ones exists, and SHALL stage one `SetFlags` per message through the backend's `stage_flags`, appended as one batch (io-pimdir's `enqueue_batch`, pimdir STORAGE §15.1) whose enqueue runs the gate of the sources' declarations once per row (§15.6), so a change refused on any message stages nothing and the failure names that message: a source declaring `mail.flags.keywords` as `none` SHALL refuse a keyword change with its detail, and one declaring it in part SHALL queue it, its receipt carrying the declaration's warning under `warnings`, never claiming the server keeps the keyword. A change on `$Important` or `$Junk` SHALL be gated by `mail.flags.important` or `mail.flags.junk` from a source stating that row, `none` included, and by `mail.flags.keywords` from one stating none (pimdir STORAGE Annex B.1), io-pimdir's gate deciding which. `list` SHALL print the flags a message carries as the overlaying reader shows them, each the shared `{raw, iana}` flag (commands: "A local store flag prints its meaning beside its spelling"), and `null` under `--json` for a message whose flags the sync engine never read.

An add or a remove SHALL match the flags a message carries as flags, not as spellings: a marker the store holds as `\seen`, or as Thunderbird's `Junk`, is the one `\Seen` or `$Junk` keeps on an add and drops on a remove. This holds for the shared flag commands on a pimdir account, which stage through the same path.

#### Scenario: A keyword is staged and read back
- GIVEN a pimdir account whose `imap` source declares `mail.flags.keywords`
- WHEN `himalaya --json pimdir flag add -m imap/INBOX -f act 42` runs
- THEN it prints `{"messages":[{"id":"42","row":7}],"warnings":[]}`, and `pimdir flag list -m imap/INBOX 42` lists `act` before the next sync

#### Scenario: A keyword kept in part
- GIVEN a pimdir account whose `gmail` source declares `mail.flags.keywords` in part, `$Important only`
- WHEN `himalaya --json pimdir flag add -m gmail/INBOX -f act 42` runs
- THEN the row is queued and the receipt prints `"warnings":["Source gmail supports mail.flags.keywords in part: $Important only"]`

#### Scenario: A source without keywords refuses one
- GIVEN a pimdir account whose `msgraph` source declares `mail.flags.keywords` as `none`
- WHEN `himalaya pimdir flag add -m msgraph/Inbox -f act 42` runs
- THEN the command fails naming `mail.flags.keywords`, the `msgraph` source and its detail, and nothing is queued

#### Scenario: A flag held in another spelling is removed
- GIVEN a pimdir message carrying `\seen` and `Junk`
- WHEN `himalaya pimdir flag remove -m imap/INBOX -f '\Seen' -f '$Junk' 42` runs
- THEN the staged set holds neither

#### Scenario: A bare IANA name is refused
- WHEN `himalaya pimdir flag add -m imap/INBOX -f seen 42` runs
- THEN the command fails naming `\Seen`, and nothing is queued

#### Scenario: A flag the store cannot hold
- GIVEN any pimdir account
- WHEN `himalaya --json pimdir flag add -m imap/INBOX -f 'to do' 42` runs
- THEN the command exits 1 with the code `flag-invalid`, and nothing is queued

## REMOVED Requirements
