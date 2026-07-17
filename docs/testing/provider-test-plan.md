# Manually testing a provider

A followable checklist to deeply exercise every **shared** command
against a real provider/account. Unit tests run each `io-*` crate in
isolation; this pass catches behaviour that only shows up end-to-end
against a live server (search semantics, date handling, error surfaces,
flag rendering, MIME round-trips).

One report is produced per `(backend, provider)` pair, e.g.
`docs/testing/imap-smtp-fastmail.md`. Follow the steps below, record
each variant, and fill in the report template at the end.

## Golden rules

- **Fake mailboxes only.** Always create a throwaway mailbox and operate
  inside it. NEVER reuse an existing mailbox: some accounts are shared
  test inboxes, others hold real mail.
- **Two fake mailboxes** are needed — `copy`/`move` want a distinct
  source and destination.
- **Clean up.** Delete the fake mailboxes at the end. For SMTP `send`,
  address the message to the account itself, then move the delivered
  copy from the inbox to Trash (find it by a unique subject marker).
- **Never print secrets.** Credentials come from the config via
  `passwd.command`/`token.command`; never echo them.

## Prerequisites

- The account is configured and `himalaya -a <account> account check`
  reports every backend `OK`.
- A built binary: `nix develop --command cargo build`, then
  `H="./target/debug/himalaya -a <account>"`.

## Fixtures

Create two fake mailboxes and populate the first with messages that
differ across every axis a command might key on — sender, subject, date,
flags, recipients, and one multipart message with an attachment:

```bash
A="Himalaya-Test-A"; B="Himalaya-Test-B"
$H imap create "$A"; $H imap create "$B"   # protocol-specific create
```

Add ~6 messages via `message add` (this doubles as the `message add`
test). Give each a distinct `Subject`, `From`, and `Date` header (use a
**correct weekday** — a wrong one is a good negative test, see report
notes), set flags on some via `-f`, and include one `multipart/mixed`
message carrying a small text attachment for the `attachment` commands.

> IMAP `APPEND` requires CRLF, but `message add` now normalises bare LF
> to CRLF, so plain Unix-LF input works from a file, inline, or stdin.

## Command checklist

Run every variant; note the outcome (pass / fail / finding). `<ID>` is
an IMAP UID, `<NAME>` a mailbox name.

### mailbox list

| Variant | Command |
| --- | --- |
| base | `mailbox list` |
| counts | `mailbox list --counts` (TOTAL/UNREAD columns) |
| width | `mailbox list --max-width 30` |
| json | `--json mailbox list` (+ `--counts`) |

### envelope list

| Variant | Command |
| --- | --- |
| base | `envelope list -m "$A"` (date-descending) |
| paging | `envelope list -m "$A" -s 2 -p 1` then `-p 2` |
| recipient | `envelope list -m "$A" -r` (TO instead of FROM) |
| attachment column | `envelope list -m "$A" --has-attachment` (ATT `@`) |
| width | `envelope list -m "$A" -w 60` |
| json | `--json envelope list -m "$A"` |
| empty mailbox | `envelope list -m "$B"` |
| default mailbox | `envelope list` (no `-m` — see default-mailbox note) |

### envelope search

Run every condition, combinator, and sort key of the query DSL:

- conditions: `from <p>`, `to <p>`, `subject <p>`, `body <p>`,
  `flag <seen|answered|flagged|draft>`, `date <yyyy-mm-dd>`,
  `after <yyyy-mm-dd>`
- combinators: `and`, `or`, `not`, and `( … )` grouping
- sort: `order by <date|from|to|subject> [asc|desc]`
- plus the same output flags as `envelope list` (`-m -p -s -w -r
  --has-attachment --json`)

Verify **which date field** `date`/`after` target (Date header vs server
arrival) and whether text matching is **substring or word-based** — both
are server-defined; record them.

### message add

| Variant | Command |
| --- | --- |
| inline `\n` escapes | `message add -m "$A" -f seen -- 'From:…\n…'` |
| inline real LF | `message add -m "$A" -- "$(printf 'From:…\n…')"` |
| file path | `message add -m "$A" -f seen -f flagged -- msg.eml` |
| stdin | `printf … \| message add -m "$A"` |
| json | `--json message add -m "$A" -- …` |
| error: no mailbox | `message add -- x` (clap error) |
| error: empty | `message add -m "$A" -- ''` |

### message read

| Variant | Command |
| --- | --- |
| pretty | `message read <ID> -m "$A"` |
| multipart | `message read <multipart-ID> -m "$A"` (text body only) |
| raw | `message read <ID> -m "$A" --raw` |
| json | `--json message read <ID> -m "$A"` |
| error | `message read 999 -m "$A"` |
| non-destructive | read an unseen msg, confirm it stays unseen (PEEK) |

### flag add / set / remove

| Variant | Command |
| --- | --- |
| add multi-flag | `flag add -m "$A" -f seen -f flagged <ID>` |
| add multi-id | `flag add -m "$A" -f answered <ID1> <ID2>` |
| set (replaces) | `flag set -m "$A" -f draft <ID>` |
| remove | `flag remove -m "$A" -f draft <ID>` |
| error: no `-f` | `flag add -m "$A" <ID>` |
| error: bad flag | `flag add -m "$A" -f bogus <ID>` |

### message copy / move

| Variant | Command |
| --- | --- |
| copy single | `message copy -f "$A" -t "$B" <ID>` |
| copy multi | `message copy -f "$A" -t "$B" <ID1> <ID2>` |
| move | `message move -f "$A" -t "$B" <ID>` (gone from source) |
| error: no `--to` | `message copy -f "$A" <ID>` |
| non-existent id | `message copy -f "$A" -t "$B" 999` (watch the message!) |
| default source | `message copy -t "$B" <ID>` (no `-f` — see note) |

### attachment list / download

| Variant | Command |
| --- | --- |
| list | `attachment list <multipart-ID> -m "$A"` |
| list inline | `attachment list <ID> -m "$A" -i` (INLINE column) |
| list none | `attachment list <plain-ID> -m "$A"` (empty) |
| download | `attachment download <ID> -m "$A" -d /tmp/att` |
| download json | `--json attachment download <ID> -m "$A" -d /tmp/att` |

### message compose / send / reply / forward (SMTP)

Save-only first (`--save "$A"`, no delivery), then one real `--send`.

- `compose`: `--from --to --cc --bcc -s --body --body-file --attach
  --signature --signature-file --save --send`. Verify the saved MIME:
  headers, signature `-- ` delimiter, attachment part.
- `send`: pipe a raw RFC 5322 message to `message send --save "$A"`.
- `reply <ID>`: check `Re:` subject, `In-Reply-To`/`References`, quoted
  body (`> …`), `-P top|bottom` posting style, `-Q` quote headline.
- `forward <ID>`: check `Fwd:` subject and the quoted original.

For every real `--send`/`send`, use a unique subject marker, then move
the delivered inbox copy to Trash during cleanup.

## Protocol-specific API (per backend)

After the shared commands, exercise the backend's raw API — for IMAP/SMTP
that is `himalaya imap …` / `himalaya smtp …`. These expose the protocol
verbatim, so the flags differ from the shared enum-driven ones.

- **IMAP** (all inside fake mailboxes): `id [-p KEY:VAL]`;
  `create`/`delete`/`rename`/`subscribe`/`unsubscribe`;
  `list` (default `LSUB` = subscribed only, `-A` = `LIST` all, `-p`
  pattern); `status`; `flags`; `select`/`close`/`unselect`
  (`close`/`unselect` need a live selection — expect an error one-shot);
  `append <mbox> -f <flag> -- <msg>`; `fetch <seq> --envelope
  --structure --flags --internal-date --size [--seq]`; `search` (every
  `--from/--subject/--since/--larger/--seen/…` key, plus `--seq`);
  `sort -S <key> [-r]`; `thread -A <algo>`; `store <seq> --action
  add|set|remove -f <flag>`; `copy`/`move <seq> <target> [--seq]`;
  `expunge`; `raw <COMMAND>`.
  - **Raw flags are literal tokens**, not the shared enum: `-f seen`
    stores a *keyword* `seen`, not `\Seen`. Pass `-f '\Seen'` for the
    system flag. Verify with `search --seen` and `fetch --flags`.
  - **`search --since/--on/--before` use `INTERNALDATE`** (arrival), not
    the `Date:` header.
- **SMTP**: `send --mail-from <addr> --rcpt-to <addr> -- <msg>` (explicit
  envelope, no header parsing); `raw <COMMAND>` (e.g. `NOOP` → `250`).
- **JMAP** (`jmap …`, all inside fake mailboxes): `query <method-calls>`
  (raw); `mailbox get/query/create/update/destroy` (`query` lists all by
  default, `--subscribed` to filter; `destroy --purge` for non-empty);
  `email get/query/read/update/delete/export/import/parse` (`update
  --add/remove-keyword|--add/remove-mailbox`); `thread get`; `identity
  get/create/update/delete` (clean up created identities!);
  `submission create --identity-id --mail-from --rcpt-to` (real send —
  the object may be transient, verify by inbox delivery);
  `vacation-response get/set` (may be `403`/unsupported per provider).
  Mailboxes are opaque ids: get them from `mailbox list --json` or
  `jmap mailbox query`.
  - **Shared `message send`/`compose --send` need `identity_id` +
    `drafts_mailbox_id`** in the account config; without them the raw
    `jmap submission create --identity-id …` is the send path.
- **Gmail** (`gmail …`, inside a fake label; the OAuth token must be
  scoped to `https://mail.google.com/`, not `carddav`): `profile get`;
  `labels list/get/create/update/delete`; `messages
  list/get/insert/import/modify/trash/untrash/send/batch-modify/batch-delete/delete`
  (`get --format metadata|raw|full`; `modify --add/remove-label`);
  `attachments get <msg> <attachmentId> -o` (needs the raw id from
  `messages get --format full`; small attachments are inlined and have
  none — use the shared `attachment download`); `drafts
  create/list/get/update/send/delete`; `threads
  list/get/modify/trash/untrash/delete`; `history list
  --start-history-id`; `settings <imap|pop|language|vacation|
  auto-forwarding> get`, `settings <send-as|filters|
  forwarding-addresses|delegates> list` (writes mutate the account —
  test with care). Labels are opaque ids: `mailbox list` /
  `gmail labels list`. Shared `-m` currently needs the label **id**, not
  its name; `message add` / `envelope search` are unsupported.

## Backend / provider-specific notes to capture

- **Text search matching**: substring vs word/token (full-text index).
  IMAP `SEARCH` is loosely specified; servers differ.
- **Date search field**: `date`/`after` may target the `Date:` header
  (sent date) or server arrival (`INTERNALDATE`).
- **Default mailbox**: whether omitting `-m`/`-f` falls back to a
  configured `mailbox.alias.inbox`, a literal `Inbox`, or errors.
- **JMAP**: mailboxes are opaque ids; the shared layer resolves display
  names to ids (`JmapClient::resolve_mailbox_id`), so names work but the
  id is what reaches the wire.
- **Flag rendering**: which symbol maps to which flag in the table
  (e.g. `*` unseen, `!` flagged).

## Cleanup

```bash
# move any delivered SMTP test messages out of the inbox
$H message move -f Inbox -t Trash <delivered-ID>
# delete the fake mailboxes
$H imap delete "$A"; $H imap delete "$B"
```

## Report template

Copy this into `docs/testing/<backend>-<provider>.md`:

```markdown
# <BACKEND> on <PROVIDER> — shared-command test report

- himalaya: <version + build rev>
- account: <name> (<backend blocks>)
- date: <yyyy-mm-dd>

## Results
<per-command pass/fail table>

## Findings
### Bugs / issues
### Provider-specific behaviour (not bugs)
### Observations

## Verdict
<release-readiness note for this backend+provider>
```
