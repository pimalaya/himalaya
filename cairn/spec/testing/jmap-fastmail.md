---
cairn: spec
capability: testing
status: current
---

# JMAP on Fastmail: unified-command test report

Unified commands on the JMAP backend. Companion: [jmap-specific-fastmail.md](jmap-specific-fastmail.md) (the `jmap …` raw API). Method: [provider-test-plan.md](provider-test-plan.md), inside two throwaway JMAP mailboxes (`live-2026-11-26-himalaya-A` `PF_N`, `-B` `PF_s`), created after the IMAP run had deleted its own.

- himalaya: release build of master 944ba0ad plus the uncommitted v3 prep, `--all-features`; `--version` prints `v3.0.0`
- account: `fastmail-jmap` (`jmap.server = https://api.fastmail.com/jmap/session`, bearer token; no identity or drafts id configured)
- date: 2026-11-26
- fixtures: the six messages of the IMAP run with their own `Message-ID`s (so Fastmail does not deduplicate them), imported with `message add`, plus one imported with `jmap email import --received-at` now while dated 3 days back (`eta`), to tell the sent and received dates apart. Every email of the run (19, the delivered and Sent copies included) was destroyed with `jmap email delete`, then the mailboxes deleted; Inbox, Sent, Drafts and Trash were 10, 7, 0 and 28 messages before and after, and a whole-account `jmap email query --subject live-2026-11-26-himalaya` finds nothing.
- live suite: not run here (the lead runs the cargo live suites).

## Results

| Command | Variants tested | Result |
| --- | --- | --- |
| `account check` | text, `--json` | ✅ |
| `mailbox list` / `create` / `delete` | `--counts`, `--json`; create of an existing name | ✅ `mailbox-duplicate`, exit 1 |
| `envelope list` | by name, by id, `-s 2 -p 2`, `-r --has-attachment`, `--json`, empty mailbox, default mailbox | ✅ |
| `envelope list` | missing mailbox | ✅ `mailbox-not-found`, exit 1 |
| `envelope search` | `from:`, `from:@example.net`, `to:`, `with:`, `cc:`, `bcc:`, `subject:`, a phrase, `body:`, a bare word, `tag:`, the `is:` flags | ✅ (word matching) |
| `envelope search` | `subject:repo`, `subject:repo*` | ✅ both empty, as on IMAP (P1, P3 of the IMAP report) |
| `envelope search` | `has:attachment` alone, beside `from:`, under `or` | ✅ served |
| `envelope search` | `date:today`, `yesterday`, a day, a month, ranges, `5h`, `10h`, `2d`, `1w`, an instant, on the root `and` | ✅ sent date re-checked: `date:5h` leaves out `eta` (received now) |
| `envelope search` | `date:5h or from:nobody`, `not date:5h`, `date:2026-11-23 or …`, `not date:2026-11-23` | ✅ reads the received date, as search.md and now `--help` say (J4) |
| `envelope search` | `not`, `-` after `--`, `or`, a group, `is:unread -date:yesterday` | ✅ |
| `envelope search` | `--sort newest-first`, `oldest-first`, `title`, `sender`, paging | ✅ |
| `envelope search` | `mid:`, `--sort relevance`, `kind:event` | ✅ refused `search-unsupported` |
| `envelope search` | `~draft`, `(from:alice`, `date:1hour` | ✅ refused `search-invalid` |
| `message add` | `-f`, `--json`, inline, stdin, by id, empty | ✅ |
| `message add` | missing mailbox | ✅ `mailbox-not-found`, exit 1 |
| `message read` | pretty, multipart, `--raw` (and `--json` `{data, mime, size}`), `--json`, `--seen`, unknown id, an id outside `-m` | ✅ `message-not-found` |
| `flag add/set/remove` | multi-flag, multi-id, set replaces, `junk` and `important`, unknown id beside a real one | ✅ item error, exit 3 |
| `message copy` / `move` | single, multi-id, unknown id, default source | ✅ item error, exit 3 |
| `message copy` / `move` | missing target, missing source | ✅ `mailbox-not-found`, exit 1 (J5 fixed) |
| `message delete` | from `B` with an unknown id beside, then from `trash` | ✅ `moved-to-trash` exit 3, then `deleted` (J7) |
| `attachment list` / `download` | `--inline`, none, `--json`, second download | ✅ `note (1).txt`, files 0600 |
| `message compose` | body-file/signature-file/attach `--save`; `--send --save` to self | ✅ |
| `message send` | `--save` of a missing mailbox | ✅ `sent: true`, item error `mailbox-not-found`, exit 3 (J5 fixed) |
| `message reply` | `--save -P top -Q`, `--send --no-save` to self | ✅ (J6) |
| `message forward` | `--save` | ✅ |
| `vacation get` | | ✅ `Enabled: no` |
| `vacation set` | `--subject --text --from --to` while disabled, then `--disable --no-from --no-to --no-subject --no-text --no-html` | ✅ prints the state; restored to the state read before the run |

## Findings

### Bugs

- **J5 (2026-11-18): fixed.** A missing mailbox is coded `mailbox-not-found` on the listings, `message add`, copy and move, and the send copy.
- None new on this surface; F5 of the IMAP report (`attachment download --stdout` without an id, uncoded) is the unified layer's and applies here too.

### Behaviour / config (not bugs)

- **J4: under `or` or `not`, a date clause reads the received date.** `date:5h or from:nobody` lists `eta` (received now, dated 3 days back) beside alpha, and `not date:2026-11-23` lists `eta`, while the root `date:2026-11-23` lists it with delta. search.md specifies it, and `envelope search --help` now names the exception ("on JMAP under `or` or `not`").
- **J6 (holds): a JMAP send always lands in Sent.** `reply --send --no-save` and the send whose `--save` failed both left a copy in Sent while their receipts say `saved: false`; the `--send --save A` copy is one email in both Sent and `A`. backends.md specifies the move.
- **J7: a removal from the trash keeps an email held elsewhere.** `message delete -m trash` of an email also in `A` printed `deleted` and only took the trash out of its `mailboxIds`, as backends.md says.
- **J2 (kept): a bogus id is an item error** coded `message-not-found`, never a silent no-op.
- Sending works from the minimal account (identity and drafts mailbox discovered, J3).
- `message add` imports with `receivedAt` equal to the `Date:` header, so a JMAP fixture added this way cannot show J4; `jmap email import --received-at` can.

## Verdict

Unified commands on JMAP are **solid**: the pimdir query language (dates re-checked on the sent date on the root `and`, `has:attachment` served), coded refusals, the send-copy receipt with exit 3 and blob downloads all pass, and J5 is fixed. J4 and J6 are specified behaviours; the help now names J4.
