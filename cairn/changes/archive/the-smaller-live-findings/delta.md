---
cairn: delta
change: the-smaller-live-findings
---

# Delta

## ADDED Requirements

### Requirement: pimdir performer takes the intents Himalaya queues
`pimdir performer <CAPABILITY> [SOURCE] [--clear]` SHALL take the capability of an intent Himalaya queues (pimdir STORAGE Annex B.2): `mail.submit`, `collection.create`, `collection.rename` or `collection.delete`. Any other SHALL be refused by the grammar, exit 2 and `usage-invalid` under `--json`, naming the ones it takes, rather than answering that no source performs it.

#### Scenario: An unknown capability
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir performer bogus.cap` runs
- THEN it exits 2 with the failure object coded `usage-invalid`, its error listing `mail.submit`, `collection.create`, `collection.rename` and `collection.delete`

### Requirement: A JMAP read is scoped to the mailbox given
The JMAP backend SHALL read one message (`message read`, and the source of `message reply|forward` and `attachment list|download`) only when the mailbox given holds it, reading its `mailboxIds` with its `blobId` in the one `Email/get`: an email the mailbox does not hold SHALL fail `message-not-found`, naming the mailbox, as on IMAP, though a JMAP id names an email account-wide. The mailbox SHALL be resolved through the mailbox listing only when it is not among the email's mailbox ids.

#### Scenario: An inbox email read under the sent mailbox
- GIVEN a JMAP account whose email `E1` the inbox alone holds
- WHEN `himalaya message read -m sent E1` runs
- THEN the command fails with the code `message-not-found`, naming `sent`, where it printed the email

## MODIFIED Requirements

### Requirement: pimdir writes name their queue row
`pimdir flag add|set|remove` SHALL print the row of each message they found and what a source supports of the change only in part: `{"messages": [{"id": string, "row": i64 | null}], "warnings": [string]}`, `null` for a message the change left as it was, for which nothing was staged, each warning once, in the words of the source's declaration (`Source gmail supports mail.flags.keywords in part: …`), `Warning:` lines after the sentence in text.

### Requirement: pimdir stages flags by their raw spelling
A source declaring `mail.flags.keywords` as `none` SHALL refuse a keyword change with its detail, and one declaring it in part SHALL queue it, its receipt carrying the declaration's warning under `warnings`, never claiming the server keeps the keyword.

#### Scenario: A keyword kept in part
- GIVEN a pimdir account whose `gmail` source declares `mail.flags.keywords` in part, `$Important only`
- WHEN `himalaya --json pimdir flag add -m gmail/INBOX -f act 42` runs
- THEN the row is queued and the receipt prints `"warnings":["Source gmail supports mail.flags.keywords in part: $Important only"]`

### Requirement: Reply to all recipients
`message reply --all` (`-A`) SHALL keep the `To` of a plain reply and fill `Cc` with the `To` and `Cc` of the source, without the user's own addresses (the reply's `From`, `--from` or else the account's `email`; the account's `email`; the user names the account's IMAP and SMTP SASL blocks transmit when they hold an `@`) and without any address already in `To`, each address once, compared ignoring case, display names kept.

#### Scenario: An account naming no email
- GIVEN an IMAP account setting no `email`, whose IMAP login is `me@icloud.com`, and a message to `me@icloud.com` and `bob@example.org`
- WHEN `himalaya message reply --all <id>` runs, or the same with `--from me@icloud.com`
- THEN the reply's `Cc` holds `bob@example.org` alone

## REMOVED Requirements

None.
