---
cairn: change
id: a-result-carries-its-errors
status: active
created: 2026-11-14
---

# A result carries its errors

## Why

A command acting on several messages or mailboxes has no way to say that some of them failed. Depending on the backend and the cause, the failed item is one of:

- **Dropped from a listing**, with a warning on stderr only. These are IMAP lines io-imap skips for not decoding (cairn/spec/backends.md, "An IMAP answer missing lines says so") and Maildir files io-maildir cannot read (`read_readable_entries`). Reported in [#801] (a MIME tree nested seven levels, whose `BODYSTRUCTURE` imap-codec refuses), [#641], [#649] and [#707].
- **Skipped from a write**, with a warning (an unknown id on Maildir, mbox and pimdir) or silently. JMAP, Gmail and Graph count an id as found only when its message is in the mailbox.
- **The end of a half-done write.** Gmail and Graph write one message per request and stop at the first failure, after the earlier ones were written.
- **Counted as not found:** a `$batch` part Graph left unanswered.

The exit status cannot tell any of these apart, since such a command exits 0. On 2026-11-06 the maintainer chose the deeper fix over an IMAP patch: listings and writes return what failed next to what succeeded, as JMAP does (RFC 8620 §5.1 `notFound`, §5.3 `notUpdated`, `notCreated` and `notDestroyed`).

## What

### Two outcomes

- **Nothing attempted.** A command that could attempt nothing (the connection, the authentication, a mailbox not found, a refused query, a missing capability) prints the failure object and exits 1, as today.
- **Several items acted on.** Such a command prints its result with an `errors` list beside it: one item error per item it could not list or act on, `[]` when there is none, in the order the items were met. A non-empty `errors` exits 3. The items listed or acted on print as when nothing failed, and `count` counts them alone.

The commands carrying `errors` are `envelope list`, `envelope search`, `mailbox list`, `flag add|set|remove`, `message copy`, `message move` and `message delete`.

The commands acting on one item keep failing whole: `message read`, `message add`, `message send`, the composers, `mailbox create|rename|delete` and `attachment list|download`. The protocol namespaces (`imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `sieve`, `smtp`, `pimdir`) keep printing what their protocol answered.

### An entry

An entry is pimalaya-cli's item error, `{id, code, error, sources}` (pimalaya/cli, cairn/changes/a-result-carries-its-errors):

- `id`: the item's id as the command prints it, the envelope id or the mailbox id. It is `null` when the backend's answer named none.
- `code`: a code of the vocabulary, `null` for a cause no caller is expected to act on.
- `error`: the message, in free wording.
- `sources`: the backend's own reason, outermost first. For example an HTTP status and Graph's `error.code`, a JMAP SetError `type`, an IMAP response code, an io-* error.

`json-schema` describes it under `himalaya-item-error`, and every command carrying `errors` describes the key as required.

### One code vocabulary

The codes stay Himalaya's single vocabulary, for item errors and failure objects alike. New:

| Code | An item, or a command, when |
|---|---|
| `message-unreadable` | the backend answered for a message the client could not read: its FETCH line or one of its items did not decode (IMAP), its object did not deserialize (JMAP), its file could not be read (Maildir) |
| `mailbox-unreadable` | a mailbox was listed but could not be read: its LIST or STATUS line did not decode (IMAP), its folder could not be read (Maildir), its counts could not be fetched |
| `permission-denied` | the server refused the item to the account: a JMAP `forbidden`, an HTTP 403 that is no rate limit (Gmail, Graph), an IMAP `NOPERM` |
| `quota-exceeded` | the write would exceed the account's quota: a JMAP `overQuota`, an IMAP `OVERQUOTA`, Graph's `ErrorQuotaExceeded` |
| `rate-limited` | the server still throttled the item after the retries: an HTTP 429, a Gmail 403 `rateLimitExceeded` or `userRateLimitExceeded`, a JMAP `rateLimit` |
| `recipient-refused` | `message send` reached nobody because the server refused a recipient: an SMTP `RCPT TO` refused (RFC 5321 §3.3), a JMAP `invalidRecipients` (RFC 8621 §7.5), the refused addresses named |

`message-not-found` widens to an item: an id given that the mailbox does not hold, as an `errors` entry, or the one message a read asked for.

`recipient-refused` codes the failure of a whole send, not an item error. Himalaya keeps sending to nobody when a recipient is refused, with io-smtp's `skip_refused_recipients` left off, since a message reaching some of its recipients surprises its sender.

### Plain output

Each entry prints as one `Error:` field of the block above a list's items, after the paging fields: `<id>  <code>  <error>`, with `-` for an absent id or code. Nothing prints under the items. A write prints its sentence, then one `Error:` line per entry.

### Per backend

What each backend can say depends on its protocol. Each was checked on 2026-11-14 against RFC 9051, RFC 2180, RFC 6851, RFC 8620, RFC 8621, RFC 5321 and the Gmail and Graph references.

- **IMAP** names a failed item only by subtraction, and its response codes belong to the command, not to one message (RFC 9051 §7.1).
  - A FETCH line that did not decode lists its message with the items that did, and a `message-unreadable` entry names the failed ones. `hasAttachment` is `null` when `BODYSTRUCTURE` failed. This rests on io-imap's partial FETCH (pimalaya/io-imap, cairn/changes/a-fetch-line-keeps-what-decodes). Until then, each skipped line becomes an entry with a `null` id.
  - A LIST line that did not decode is `mailbox-unreadable` with a `null` id. A STATUS line that did not decode is `mailbox-unreadable` naming its mailbox, whose counts print `null`.
  - A message a listing's window did not return is no error: the mailbox changed since its `SELECT` (RFC 2180 §4.1).
  - A UID a write was given that is absent from both the `UID STORE` echoes and a `UID SEARCH` on the set is `message-not-found`.
  - `COPY` and `MOVE` answer for the whole set (RFC 9051 §6.4.7, RFC 6851 §3.3). A refused one stays a whole failure, coded by its response code.
- **JMAP** reports each item, with a type.
  - An object that did not deserialize is `message-unreadable`.
  - An id `Email/set` or `Email/copy` names in `notUpdated`, `notDestroyed` or `notCreated` takes the code its SetError type maps to: `notFound` to `message-not-found`, `forbidden` to `permission-denied`, `overQuota` to `quota-exceeded`, `rateLimit` to `rate-limited`, and any other to `null` with its type in `sources`.
  - An id the chained `Email/get` reports in `notFound` is left out of a listing, the email being gone since the query.
  - This rests on pimalaya/io-jmap, cairn/changes/a-get-keeps-what-it-misses.
- **Gmail** reports each call of a batch on its own (developers.google.com/workspace/gmail/api/guides/batch).
  - A listing reads its metadata through `GmailBatch`, 100 per request, where it sends one `messages.get` each today. A part answered 404 is left out, since its message was deleted after the list was read. A part still throttled after the retries is `rate-limited`, another 403 `permission-denied`, and an unanswered part an entry coded `null`.
  - A write looks its ids up through batched `messages.get` (minimal). An id Gmail does not know, or whose message does not carry the mailbox's label, is `message-not-found`.
  - A write then goes through batched `messages.modify`, `trash` and `delete` (pimalaya/io-gmail, cairn/changes/a-batch-carries-writes). Until that change, it sends one request per message, carrying on past a failed one.
- **Microsoft Graph** reports each request of a `$batch` (learn.microsoft.com/graph/json-batching).
  - A listing is one request, so it fails whole.
  - A write looks its ids up through `$batch` as today, then patches, copies, moves and deletes through `$batch` too, 20 requests at a time, with the existing retries.
  - A part answered 400 or 404 is `message-not-found`, 403 `permission-denied`, 429 after the retries `rate-limited`. Any other is `null`, with Graph's `error.code` in `sources`.
  - A part the answer left out is an entry, no longer an id not found.
- **Maildir** reads and writes one file per message.
  - A file that cannot be read is `message-unreadable` with its id, and a folder that cannot be read is `mailbox-unreadable` with its name (pimalaya/io-maildir, cairn/changes/a-listing-returns-what-it-could-not-read).
  - An unknown id is `message-not-found` instead of a warning.
- **mbox** rewrites a file whole under its lock (cairn/spec/backends.md), so a refused write stays a whole failure. A listing reads one file, and an unreadable range fails it whole. An unknown id is `message-not-found` instead of a warning.
- **pimdir**:
  - An unknown id is `message-not-found` instead of a warning.
  - A write queues as one transaction, refused whole.
  - io-pimdir's write notes stay warnings, since pimdir details stay out of the unified outputs (cairn/spec/commands.md). A row the sync engine parks later is what `pimdir queue list` reports.
- **SMTP** and **ManageSieve** act on one item per command.

io-msgraph, io-mbox, io-pimdir, io-smtp and io-managesieve need no change. Graph's `$batch` already parses each part into an `MsgraphApiError { status, code, message }`, and io-smtp already returns the recipients it refused.

### MCP

A result with errors exits 3, a partial outcome, which `himalaya mcp` already returns as `structuredContent` beside `isError: true`. An agent reads the items and their `errors` together.

## Scope / non-goals

- `account check` keeps its rows `{backend, ok, code, error}`, the shared noun, and its exit 3.
- No retry beyond the existing ones (Graph's `BATCH_RETRIES`).
- The protocol namespaces keep their outputs.
- So do the keywords io-maildir drops when it cannot store them. A later change can report them the same way.
- imap-codec cannot tell which item of a FETCH line failed, nor be told a deeper body nesting. io-imap opens an upstream issue (its change); until then, io-imap's own scanner is what recovers the items.

## Decisions

- **`errors`, one list, codes from one vocabulary.** The maintainer chose this on 2026-11-14.
- **None found still prints the result.** A write whose ids are all unknown prints `count: 0` and one `message-not-found` entry per id, exit 3, where today it prints the failure object, exit 1. The rule stays one: an item's failure is an entry, even when every item fails.
- **A listing reports what it could not read, never what is gone.** A message the server stopped listing between two requests is the mailbox changing, not a failure.
- **v3.** v3 is unreleased and its exit 3 landed on 2026-11-13, so `errors` and its exit 3 ship in v3, not in a later major.

## Order

1. pimalaya-cli's `ItemError`, then its release.
2. Himalaya's `errors` on every listed command, filled from what the backends give today: IMAP's skipped lines as entries with a `null` id, unknown ids on every backend, Gmail and Graph carrying on past a failed write, and Graph's unanswered parts.
3. Each backend change as its crate releases: io-imap (partial FETCH), io-jmap (typed SetErrors, objects that do not deserialize, `invalidRecipients`), io-maildir (unreadable entries and folders) and io-gmail (batched writes).
4. The imap-codec issue, which only adds precision.

[#801]: https://github.com/pimalaya/himalaya/issues/801
[#641]: https://github.com/pimalaya/himalaya/issues/641
[#649]: https://github.com/pimalaya/himalaya/issues/649
[#707]: https://github.com/pimalaya/himalaya/issues/707
