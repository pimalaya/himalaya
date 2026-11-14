---
cairn: tasks
change: a-result-carries-its-errors
---

# Tasks

## The contract

- [ ] Take pimalaya-cli's `ItemError` once released (still a git pin, 0be1b1f, in Cargo.toml [patch.crates-io])
- [x] src/error.rs: `message-unreadable`, `mailbox-unreadable`, `permission-denied`, `quota-exceeded`, `rate-limited`, `recipient-refused`; `message-not-found` documented for an item
- [x] `errors` on `EnvelopeListOutput` (list and search), `MailboxListOutput`, the flag outputs, `MessageCopyOutput`, `MessageMoveOutput`, `MessageDeleteOutput`; `count` counting what was acted on; exit 3 through `PartialOutcome` when not empty
- [x] Text: `Error:` fields in the listing block after the paging fields; one `Error:` line after a write's sentence
- [x] json-schema: `himalaya-item-error`; `errors` required where printed; the schema walk test
- [x] Every id-taking unified write: an unknown id an entry, none found `count: 0` with exit 3

## The backends, from what they give today

- [x] IMAP: a skipped line as an entry with a `null` id (`message-unreadable` for FETCH, `mailbox-unreadable` for LIST, STATUS naming its mailbox); a UID neither echoed nor found by `UID SEARCH` as `message-not-found`; `message read` on an undecodable answer coded `message-unreadable`
- [x] JMAP: SetError types mapped to codes (typed since io-jmap 26a77bb); `notFound` of a listing left out
- [x] Gmail: listing through `GmailBatch`; ids looked up through `GmailBatch`; writes carrying on past a failed message
- [x] Graph: writes through `batch_all`; an unanswered part as an entry; statuses mapped, a 400 not found only with `ErrorInvalidIdMalformed`, the id lookup too
- [x] Maildir, mbox, pimdir: unknown ids as entries instead of warnings
- [x] `message send`: `recipient-refused` on a refused `RCPT TO` (SMTP) and `invalidRecipients` (JMAP)

## The backend changes, as their crates release

- [x] io-imap partial FETCH: the message listed with what decoded, the entry naming its UID and failed items
- [x] io-jmap: typed SetErrors, objects that do not deserialize as `message-unreadable`, `invalidRecipients` naming the addresses
- [x] io-maildir: unreadable entries and folders as entries
- [x] io-gmail: writes through batched `messages.modify`, `trash` and `delete` (`trash` stays a `messages.modify` adding `TRASH`; see the proposal's Decisions)

## Proof

- [x] Tests per backend with a failing item: IMAP against a test server answering a line that does not decode, Graph and Gmail against recorded batch answers, Maildir with an unreadable file, JMAP with a `notUpdated` map
- [x] #801 reproduced on a scratch Dovecot mailbox: the depth-7 message listed with `hasAttachment: null` and its entry
- [x] Clippy over the feature sets of tests.yml; `cargo test --all-features`; Maildir alone
- [x] CHANGELOG; fold the delta into cairn/spec/commands.md, backends.md and mcp.md; log; status landed
- [x] Live: the Google, Microsoft and Fastmail suites; `gmail_writes` and `graph_writes` add, flag, move and delete a throwaway message beside a bogus id
