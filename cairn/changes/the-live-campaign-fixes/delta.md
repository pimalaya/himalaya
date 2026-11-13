---
cairn: delta
change: the-live-campaign-fixes
---

# Delta

The requirements below are folded as they now read in the spec; each entry names its capability and what changed.

## ADDED Requirements

### Requirement: iCloud SORT ignores its keys
(provider-quirks) iCloud answers every `SORT` in UID order; a server advertising `X-APPLE-REMOTE-LINKS` sorts client-side when `imap.sort.fallback` is unset.

### Requirement: JMAP writes are scoped to the mailbox given
(backends) Flags, copy, move and delete read the ids' `mailboxIds` first; a move onto its own mailbox changes nothing; a trash removal destroys only what the trash alone holds; an `Email/set` refusal other than `notFound` fails, saying how many were written.

### Requirement: A JMAP duplicate import files the existing email
(backends) On `alreadyExists`, an add or a save files the existing email and reports its id, a send submits it; any other refusal names its type and description.

### Requirement: A Gmail answer joins its parent's thread
(backends) A message added or sent with `In-Reply-To:` carries the `threadId` of the first parent the account holds.

## MODIFIED Requirements

### Requirement: IMAP special-use roles come from a plain LIST
(provider-quirks) Renamed from "IMAP special-use is inbox-only for now": the roles a plain `LIST` carries are read and resolve role words, aliases winning; `RETURN (SPECIAL-USE)` stays unissued.

### Requirement: An unserved verb is refused before connecting
(commands) A `--backend` naming an unconfigured block fails with `capability-missing`.

### Requirement: Raw passthrough is byte-verbatim
(commands) The `imap raw` and `sieve raw` events print under `--json` only; their text form prints every response line.

### Requirement: account check covers every declared backend
(commands) A Maildir or m2dir block is checked by listing its store.

### Requirement: Text output is plain text
(commands) A Gmail snippet prints without `<b>` marks and with its HTML entities decoded.

### Requirement: Data commands serialize their data
(commands) `gmail messages get` serializes its `payload` under `--format full` and lists the leaf parts in text.

### Requirement: Composers default the From header
(commands) The JSON template carries `attachments` and an LF body; the body ends with a line break, a quoted empty line is `>`; `--no-save` needs `--send`; an attached message is never base64.

### Requirement: Raw message input is shared
(commands) The positional comes last, `--` only before a hyphen; a value naming no file must open on a header field; flag options take one value per occurrence.

### Requirement: Saving a sent message follows the send
(commands) Gmail files the saved copy by labelling the message it sent.

### Requirement: A sent reply or forward flags its source
(commands) `answered` is skipped on a backend declaring `flags.answered` `none`.

### Requirement: A forward carries the attachments of its source
(commands) Each attachment keeps its `Content-Type` parameters.

### Requirement: A read message is a designed view
(commands) The text From, To and Cc lines show the unreadable entries.

### Requirement: A part can be read without touching the disk
(commands) Without an id, `attachment download` writes the inline parts only with `--inline`.

### Requirement: A command given message ids says how many it found
(commands) Ids are required and deduplicated, a trash removal counted, ids scoped on JMAP and Gmail, looked up before the first write on the local stores, receipts spelling flags as written.

### Requirement: Unified mailbox writes
(commands) NAME validated, existing targets and mailboxes holding others refused, Gmail sublabels renamed, outputs naming the full name and the listed id.

### Requirement: The auto-reply is in the unified API
(commands) `vacation set` with no option is a usage error.

### Requirement: A write prints what it acted on
(commands) `message add` prints `saved`.

### Requirement: Listings resume from their cursor
(commands) `--page 0` and a page token `0` are refused.

### Requirement: An IMAP command prints its completion
(commands) `select` prints `firstUnseen`; `thread` without `THREAD=` the algorithm is refused.

### Requirement: IMAP flag counts
(backends) A trash removal counts the same way.

### Requirement: Local storage backends
(backends) m2dir copies and moves with flags, writes the `$` spelling, names by decoded path, skips a bad checksum; Maildir skips plain files, fails on a missing root, refuses flags into `new`.

### Requirement: mbox backend
(backends) Plain-layout parents refused, Thunderbird .sbd directory moved and guarded, a root that is no directory refused.

### Requirement: An mbox message is its content id
(backends) A read leaves out the store's fields.

### Requirement: Maildir surfaces custom keywords on demand
(backends) `flags.important` and `flags.junk` are `none` without `maildir.keywords.dovecot`.

### Requirement: Mailbox role
(backends) IMAP reads roles, role names address mailboxes on IMAP, a delete without a trash fails with `capability-missing`, duplicate and unknown names are refused.

### Requirement: One query language
(search) `order by` is words outside a pattern, keywords whole words, quoting and escapes defined, blanks inside parentheses insignificant.

### Requirement: A query that does not parse is refused with a code
(search) One report over the whole query, naming what was found and expected.

### Requirement: What a backend searches is declared
(search) IMAP text matching is `partial`; Gmail's `attachment` is `partial`.

### Requirement: Per-backend translation
(search) IMAP negations on single keys, canonical keywords, duplicate `SORT` UIDs dropped, iCloud fallback; `sort_envelopes` follows RFC 5256; a Gmail pattern is a literal.

### Requirement: A tool's input is the command's grammar
(mcp) A lone value stands for an array of one; integers typed.

### Requirement: A tool reads and writes no local file
(mcp) A message opening on no header field is refused.

### Requirement: An attachment is read as an embedded resource
(mcp) `inline` is no property; `attachment-id` is a string.

## REMOVED Requirements

None.
