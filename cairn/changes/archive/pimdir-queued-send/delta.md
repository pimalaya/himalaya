---
cairn: delta
change: pimdir-queued-send
---

## ADDED Requirements

### Requirement: pimdir sends by queueing a submit intent
The pimdir backend SHALL send a message by staging its raw bytes and enqueueing one `submit` action, which whatever owns the store performs through its own channel. It SHALL NOT open a network connection to send, and SHALL NOT use the account's `smtp` section.

The payload SHALL be `v: 1` JSON carrying `object` (the body hash), `from`, `rcpts` and `subject`, derived from the headers by the same function the SMTP adapter uses: the first `From:` address, every `To:`, `Cc:` and `Bcc:` address, and the decoded subject. A message with no `From:` or no recipient SHALL be refused before anything is staged. The body SHALL be stored as given, `Bcc:` included; removing it is the sending channel's job.

The row SHALL anchor on the `--save` mailbox when one is given, else on the mailbox `mailbox.alias.sent` names; with neither, the send SHALL be refused, naming the alias to set, rather than create a collection.

The command SHALL report the queue row id, and its text SHALL say the message is queued for sending, not sent.

#### Scenario: A send made offline waits in the queue
- GIVEN a pimdir account with `mailbox.alias.sent` set and no network
- WHEN a message is sent
- THEN one `submit` row is queued on the sent mailbox with the message's envelope, and the command prints its row id

#### Scenario: A Bcc recipient is in the envelope
- GIVEN a message with `To: a@x.org` and `Bcc: b@x.org`
- WHEN it is sent on a pimdir account
- THEN the payload's `rcpts` holds both addresses and the stored body still carries `Bcc:`

#### Scenario: No anchor is refused
- GIVEN a pimdir account with no `mailbox.alias.sent`
- WHEN a message is sent without `--save`
- THEN nothing is staged and the error names `mailbox.alias.sent`

### Requirement: The queue view shows queued sends
`himalaya pimdir queue list` SHALL render a queued `submit` as a message, derived from the body it pins as a create is, marked as a send, beside the queued creates of the same mailbox. The count an envelope listing reports SHALL include the mailbox's queued sends.

## MODIFIED Requirements

### Requirement: Sending transport
Backends that self-send (JMAP, Gmail, Graph) SHALL route `send_message` through their own API, and pimdir through its queue. Storage backends that cannot send (IMAP, Maildir, m2dir, mbox) SHALL send through the account's SMTP transport, adapted in `src/smtp/backend.rs` over io-smtp, which parses the RFC 5321 envelope from the raw message headers. The transmitted message SHALL NOT carry the `Bcc:` field (RFC 5322 3.6.3), which io-smtp removes after the envelope is derived. The protocol-level `smtp send` SHALL transmit its message verbatim, its envelope being explicit.
