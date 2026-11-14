---
cairn: tasks
change: a-result-carries-its-errors
---

# Tasks

## The contract

- [ ] Take pimalaya-cli's `ItemError` once released
- [ ] src/error.rs: `message-unreadable`, `mailbox-unreadable`, `permission-denied`, `quota-exceeded`, `rate-limited`, `recipient-refused`; `message-not-found` documented for an item
- [ ] `errors` on `EnvelopeListOutput` (list and search), `MailboxListOutput`, the flag outputs, `MessageCopyOutput`, `MessageMoveOutput`, `MessageDeleteOutput`; `count` counting what was acted on; exit 3 through `PartialOutcome` when not empty
- [ ] Text: `Error:` fields in the listing block after the paging fields; one `Error:` line after a write's sentence
- [ ] json-schema: `himalaya-item-error`; `errors` required where printed; the schema walk test
- [ ] Every id-taking unified write: an unknown id an entry, none found `count: 0` with exit 3

## The backends, from what they give today

- [ ] IMAP: a skipped line as an entry with a `null` id (`message-unreadable` for FETCH, `mailbox-unreadable` for LIST, STATUS naming its mailbox); a UID neither echoed nor found by `UID SEARCH` as `message-not-found`; `message read` on an undecodable answer coded `message-unreadable`
- [ ] JMAP: SetError types mapped to codes (`Other { type_name }` read by name until io-jmap types them); `notFound` of a listing left out
- [ ] Gmail: listing through `GmailBatch`; ids looked up through `GmailBatch`; writes carrying on past a failed message
- [ ] Graph: writes through `batch_all`; an unanswered part as an entry; statuses mapped
- [ ] Maildir, mbox, pimdir: unknown ids as entries instead of warnings
- [ ] `message send`: `recipient-refused` on a refused `RCPT TO` (SMTP) and `invalidRecipients` (JMAP)

## The backend changes, as their crates release

- [ ] io-imap partial FETCH: the message listed with what decoded, the entry naming its UID and failed items
- [ ] io-jmap: typed SetErrors, objects that do not deserialize as `message-unreadable`, `invalidRecipients` naming the addresses
- [ ] io-maildir: unreadable entries and folders as entries
- [ ] io-gmail: writes through batched `messages.modify`, `trash` and `delete`

## Proof

- [ ] Tests per backend with a failing item: IMAP against a test server answering a line that does not decode, Graph and Gmail against recorded batch answers, Maildir with an unreadable file, JMAP with a `notUpdated` map
- [ ] #801 reproduced on a scratch Dovecot mailbox: the depth-7 message listed with `hasAttachment: null` and its entry
- [ ] Clippy over the feature sets of tests.yml; `cargo test --all-features`; Maildir alone
- [ ] CHANGELOG; fold the delta into cairn/spec/commands.md, backends.md and mcp.md; log; status landed
