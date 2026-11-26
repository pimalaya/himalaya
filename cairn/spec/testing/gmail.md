---
cairn: spec
capability: testing
status: current
---

# Gmail REST on Google: unified-command test report

Unified commands on the Gmail REST backend. Companion: [gmail-specific.md](gmail-specific.md) (the `gmail …` raw API). Method: [provider-test-plan.md](provider-test-plan.md), inside throwaway labels.

- himalaya: release build of master 944ba0ad plus the uncommitted v3 prep, `--all-features`; `--version` prints `v3.0.0`
- account: `google` (Gmail REST API, `gmail.auth.token.command`), the pimalaya.org Workspace test mailbox, which holds a fake corpus kept untouched
- date: 2026-11-26
- fixtures: labels `live-2026-11-26-himalaya-A` (`Label_584`), `live-2026-11-26-himalaya-B` (`Label_585`), the sublabel `live-2026-11-26-himalaya-B/sub` (`Label_586`, made and deleted for the rename cell) and `live-2026-11-26-himalaya-T` (`Label_587`, 120 messages for the unpaged listing); six messages added with `message add` dated relative to the run (2 hours, 3 hours, 8 hours, yesterday, 3 days, 10 days ago, the first a `multipart/mixed` with `note.txt`), subjects starting `live-2026-11-26-himalaya`; sends to the account's own address only. Every message (133, with the gmail-specific ones) and label the run made was deleted at the end; the profile counts were 502 messages and 434 threads before and after, and the trash held its one earlier message.
- live suite: not run in this pass (the lead runs the cargo live suites).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `account check` | text, `--json` (row `tls` all `null`) | ✅ |
| `mailbox list` | `--counts`, `--json` | ✅ |
| `mailbox create` | top level, `--parent`, existing name, name holding `/` | ✅ existing `mailbox-duplicate`, exit 1; `/` refused before any request; `--parent` prints `name` as given (`sub`) |
| `mailbox rename` / `delete` | by id and by name, sublabel renamed along, onto an existing label, missing label | ✅ missing `mailbox-not-found`, exit 1; onto an existing label refused uncoded (note) |
| `envelope list` | by name, by id, `-s 2 -p 1`, `-s 2 -p 2`, `-r`, `--json`, empty label, no `-m` (inbox), missing label, `--page 0` | ✅ `mailbox-not-found` exit 1, `usage-invalid` exit 2 |
| `envelope list --has-attachment` | text, `--json` | ✅ `hasAttachment` `null` on every row, as backends.md says for Gmail |
| `envelope list -s 0` | 120 messages; `-s 100` then `--page-token 2` | ✅ all 120, `nextPage` `null`; 100 with `nextPage` `"2"`, then 20 |
| `envelope search` dates | `date:today`, `date:yesterday`, `date:2d`, `date:5h`, `date:2026-11-20..`, `date:..2026-11-20`, `date:2026-11-23`, `not date:2d`, `-- -date:2d`, `date:5h or from:eve` | ✅ |
| `envelope search` fields | `from:alice`, `to:bob`, `with:carol`, `cc:carol`, `subject:"weekly report"`, `subject:report`, `subject:repo` (none: words), `body:zebra`, bare `zebra` and `giraffe`, `is:unread`, `is:read`, `is:flagged`, `is:draft`, `has:attachment`, `mid:` bare and in angle brackets, `tag:$Important` | ✅ |
| `envelope search` combinators | `is:flagged or is:unread`, `from:alice (subject:report or has:attachment)`, `(from:alice subject:report) or (from:bob subject:lunch)`, `-- -from:alice` | ✅ |
| `envelope search` order and output | `--sort newest-first`, `oldest-first`, `title`, `sender`, `-s 2 -p 2`, text with `-r --has-attachment`, no query, `-s 0` over 120, missing label | ✅ |
| `envelope search` refusals | `is:answered`, `tag:custom`, `--sort relevance` (`search-unsupported`), `date:15-11-26`, `~draft` (`search-invalid`) | ✅ exit 1 |
| `envelope search` prefix | `subject:repo*`, `zebr*` | ❌ no hits, exit 0: the `*` is dropped (H1) |
| `message add` | file with `-f seen`, inline message, stdin, by label id with `-f seen -f flagged`, text, `--json` | ✅ |
| `message add` refusals | `-f answered`, no `-m`, empty message, missing file | ✅ `capability-missing` exit 1, `usage-invalid` exit 2, uncoded exit 1 |
| `message read` | pretty, multipart, `--raw`, `--json`, unknown id, another label, unread kept, `--seen` | ✅ `message-not-found` exit 1 |
| `flag add/set/remove` | seen and flagged, important on two ids, set, remove two flags, unknown id beside a real one, no `-f`, `bogus` | ✅ item error exit 3, `usage-invalid` exit 2; answered and draft refused `capability-missing` |
| `message copy` / `move` | single, multi-id, by label id, unknown id, no `--to`, missing target label | ✅ item error exit 3; missing label `mailbox-not-found` exit 1 (G6 fixed) |
| `message delete` | to the trash with an unknown id beside, then from the trash | ✅ `moved-to-trash` with an item error, exit 3; `deleted` |
| `attachment list` / `download` | text, `--json`, `--inline`, plain message, second download, `--stdout`, unknown part | ✅ `note (1).txt`, files 0600; unknown part uncoded exit 1 |
| `message compose --save` | `--from --to --cc --bcc --body --attach --signature`, save only | ✅ `-- ` delimiter, base64 `a.txt` part |
| `message compose` | neither save nor send, `--save` with `--no-save` | ✅ template printed; `usage-invalid` exit 2 |
| `message compose --send --save` | to self | ✅ one message carrying `INBOX`, `SENT` and the label |
| `message send --save` | piped raw message to self; missing label | ✅ `sent: true`; missing label an item coded `mailbox-not-found`, exit 3 (G6 fixed) |
| `message reply --send --no-save` | `-P bottom -Q`, to self | ✅ `Re:`, `In-Reply-To` and `References`, quoted body |
| `message forward --save` | `-t` self | ✅ `Fwd:`, `References`, quoted original; no `From` (note) |
| `vacation get` | text, `--json` | ✅ off |
| `vacation set` | | NOT RUN: the rc's Google token lacks the `https://www.googleapis.com/auth/gmail.settings.basic` scope (owner to add it and re-consent) |
| rate limit | per-minute quota | NOT RUN: the quota was not met in this run's searches and listings, so the `rate-limited` path went unexercised |

## Findings

### Bugs

- **H1: a trailing `*` is dropped on Gmail, so a prefix word finds nothing.** `himalaya -a google --json envelope search -m live-2026-11-26-himalaya-A 'subject:repo*'` printed `{"envelopes":[],"nextPage":null,"total":null,"errors":[]}`, exit 0, while `subject:report` found the `weekly report` fixture; `zebr*` found nothing where `zebra` found it. The help says "A trailing `*` makes a word a prefix" and search.md says no term is dropped, yet `lower` in src/unified/email/search/query.rs reads `text.text` and never `text.prefix`, so Gmail receives the whole word `repo`. Either refuse the prefix with `search-unsupported` (Gmail has no prefix operator) or declare it; the shared lowering suggests Graph loses it too (not tested here).
- **H1 fixed 2026-11-26** (cairn/changes/archive/a-prefix-or-phrase-is-never-dropped), rechecked on the release build in `INBOX`: `subject:integ*`, `integ*` and `from:goo*` exit 1 coded `search-unsupported` (``Cannot search `subject:integ*`: Gmail search matches whole words and has no prefix search``) before any search; `subject:"integration test"` lists the 56 `io-* integration test` messages as `subject:integration` does, `subject:"test integration"` none.
- **G6 (2026-11-18) is fixed:** a missing label is now `mailbox-not-found` on `message move` (exit 1) and on `message send --save` (item error, exit 3).

### Behaviour (not bugs)

- `hasAttachment` is `null` on every Gmail listing row, even with `--has-attachment` (backends.md: "Gmail SHALL leave it `null`"); `message read --json` reports `true` for the multipart fixture.
- Renaming a label onto an existing one is refused uncoded: `{"code":null,"error":"Gmail API returned HTTP 409: Label name exists or conflicts",…}`, exit 1. commands.md codes only a create's duplicate (`mailbox-duplicate`), so this is a gap, not a contract break.
- The test rc's `google` account sets no `email`, so `message forward` and a template without `--from` carry no `From:` and their `Message-ID` falls back on the machine's hostname, as commands.md specifies; Gmail fills `From:` itself on a send. Setting `email` in the rc would avoid the hostname.
- Gmail replaces the `Message-ID` of a sent message with its own; the reply's `In-Reply-To` names Gmail's id, which is what the source then carries.
- Gmail has no `answered` label and refuses `DRAFT` from a client: `message add -f answered`, `flag add -f answered|draft` and `envelope search is:answered` are refused before any write. A reply's source is therefore not flagged on Gmail.
- A message sent to the account itself is one message carrying `INBOX` and `SENT`; `compose --send --save <label>` labels that message rather than adding a copy.
- An inserted message gains a `Received:` header from `gmailapi.google.com`, which `message read --raw` shows verbatim.
- Gmail date clauses compare the received date (`partial`); the inserted fixtures took their `Date:` as the internal date, so the day and relative cells matched as on the local backends, `date:5h` to the hour.
- Text matches whole words: `subject:repo` does not find `report`; `tag:` answers `$Important` alone and refuses other keywords.

## Verdict

Unified commands on Gmail are **solid** on v3: the pimdir query language translates correctly for every field, combinator and order tried, the unpaged listing and page tokens read every page, writes report unknown ids per item, and G6 is fixed. One bug: a prefix word (`repo*`) silently matches nothing (H1). The rate-limited path and the settings write were not exercised this run.
