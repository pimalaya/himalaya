---
cairn: delta
change: the-family-releases-adopted
---

## ADDED Requirements

### Requirement: An IMAP answer missing lines says so (backends)
An IMAP answer holding untagged lines io-imap skipped for not decoding SHALL be taken with a warning counting them and naming the command, in the unified commands and the `imap` ones alike: the listing, the search or the counts built from it may be incomplete. A `message read` whose `FETCH` answer held such a line and no message SHALL fail saying the answer does not decode, never as `message-not-found`. The text of a server `ALERT` SHALL be logged as a warning.

#### Scenario: A listing with a line that does not decode
- GIVEN a mailbox one of whose `FETCH` lines the client cannot decode
- WHEN `envelope list` reads it
- THEN the other envelopes print and a warning says one line of the FETCH response was skipped

## MODIFIED Requirements

### Requirement: IMAP copy and move counts (backends)
Added: a `COPYUID` counting more messages than UIDs were asked SHALL be read as none sent, its two sets being ranges a server is free to announce in the billions.

### Requirement: mbox backend (backends)
Added: a name that is empty, holds a NUL or a `.` or `..` level, or a level ending in `.lock` or `.msf`, SHALL be refused by every command taking a mailbox, the reads included, io-mbox resolving it to no file.

### Requirement: mbox flags interoperate (backends)
Added: a keyword `X-Keywords` would not give back, one that is empty, longer than 255 bytes or holding a whitespace, a comma or a control character, SHALL be refused before any write.

### Requirement: mbox reads go through a cache (backends)
Added: a flag change or a removal the file refuses as stale, before anything is written, SHALL rebuild the index, plan its edits again from it and write once more, the file then being scanned whole under the lock.

### Requirement: Sending transport (backends)
Added: an envelope address holding a control character SHALL fail the send wherever the envelope is derived from the headers, the pimdir queue included, and an address io-smtp refuses as an RFC 5321 mailbox (a space, a line break, a second `@`) SHALL fail an SMTP send before `MAIL FROM`, the error naming the address with its control characters escaped. `smtp send` SHALL refuse a `--mail-from` or `--rcpt-to` io-smtp refuses when the command line is read, a quoted local part and an address literal being taken.

### Requirement: A JMAP send files the message as sent (backends)
Added: a patch the server refuses, or answers with an error, once the message is submitted SHALL be logged as a warning saying the message stays a draft, the send having succeeded.

### Requirement: An IMAP command prints its completion (commands)
`copy` and `move` print the `COPYUID` as `uidValidity`, and `sourceUids` beside `destinationUids`, each source UID at the place of the UID it got; it prints `null` and empty without UIDPLUS or when it counts over a million messages.

### Requirement: Feature-gated backends (packaging)
Added: the `imap`, `smtp` and `sieve` features SHALL enable the `scram` feature of io-imap, io-smtp and io-managesieve, so `scram-sha-256` runs wherever the configuration takes it.

## REMOVED Requirements

None.
