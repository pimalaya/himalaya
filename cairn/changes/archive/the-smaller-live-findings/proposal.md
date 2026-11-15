---
cairn: change
id: the-smaller-live-findings
status: landed
created: 2026-11-15
---

# The smaller live findings

## Why

Task H32 of the family's cli-alignment-2026-11-15 plan, the low findings of live testing on 2026-11-15 (live-testing-2026-11-15, reports/himalaya/part2-pimdir.md, imap-smtp-icloud.md, jmap-fastmail.md):

- `pimdir performer bogus.cap` exited 0 with `{"candidates":[],"chosen":null}` (part2-pimdir.md, O3).
- A Gmail keyword was queued "in part" (`mail.flags.keywords` partial, `$Important only`), the warning in the log at WARN alone, then dropped by Gmail; neverest's sync report called the push done (part2-pimdir.md, P2, O9).
- `message reply --all` on an iCloud account setting no `email` kept the user's own address in `Cc`, even with `--from` naming it (imap-smtp-icloud.md, O2).
- `message read -m sent <id>` on JMAP printed an email the inbox alone holds, where IMAP answers `message-not-found` (jmap-fastmail.md, O2).
- `message read` of an unknown id on pimdir and JMAP carried `code: null` (part2-pimdir.md, O2; jmap-fastmail.md, O3): landed already in a-server-refusal-codes-the-command (task H3), which the live run predates.

## What

- `pimdir performer` takes the four intents Himalaya queues (`mail.submit`, `collection.create`, `collection.rename`, `collection.delete`) and the grammar refuses any other, `usage-invalid`, listing them.
- `pimdir flag add|set|remove` print `warnings`: what a source supports of the change only in part, in its declaration's words, each once, `Warning:` lines in text. Himalaya never said pushed; the receipt now says the server may keep less. Refusing a keyword Gmail cannot keep is neverest's to declare (`mail.flags.keywords` `none` beside its own `mail.flags.important` `full`), which io-pimdir's gate then refuses up front, coded `unsupported`, on every path.
- A reply to all leaves out the reply's own `From` (`--from`, else `email`) and the account's logins: the user names its IMAP and SMTP SASL blocks transmit holding an `@` (PLAIN's `authzid` included), read before the client takes the blocks. The source's `Delivered-To` is not read, a header not every server adds.
- A JMAP read asks `mailboxIds` with `blobId` and fails `message-not-found`, naming the mailbox, when the mailbox given does not hold the email; the mailbox listing is read only when the mailbox is not among the email's ids. Gmail and Graph reads stay unscoped (left open: Graph would need a request more).
