---
cairn: spec
capability: testing
status: current
---

# IMAP + SMTP on Fastmail: unified-command test report

- himalaya: release build of master 944ba0ad plus the uncommitted v3 prep, `--all-features`; `--version` prints `v3.0.0`
- account: `fastmail` (`imaps://imap.fastmail.com`, `smtps://smtp.fastmail.com`, SASL PLAIN; Cyrus IMAP 3.15.0-alpha0, Fastmail build of 2026-11-23); the scratch TLS copies of 2026-11-18 (`fastmail-tls`) are not in the live rc
- date: 2026-11-26
- method: every unified command and flag variant, inside throwaway mailboxes `live-2026-11-26-himalaya-A`, `-B` and `-C`/`-C2` (a rename), per [provider-test-plan.md](provider-test-plan.md). Fixtures: six messages with subjects starting `live-2026-11-26-himalaya`, dated 3 hours, 8 hours, 1 day, 3 days, 5 days and 10 days before the run (correct weekdays), one `multipart/mixed` with `note.txt` and a `To:` group. Every message delivered to the inbox was moved into `A`, a copy added to Sent (the role read) was deleted through Trash, and the mailboxes were deleted at the end. Inbox, Sent, Drafts and Trash were 10, 7, 0 and 28 messages before and after the run, and `envelope search subject:live-2026-11-26-himalaya` finds nothing in any of them.

## Results

| Command | Variants tested | Result |
| --- | --- | --- |
| `account check` | text, `--json` | ✅ `OK, TLS` per backend, `tls` all `null` (build defaults) |
| `account check` | account `tls.cert` | NOT RUN: no such account in the live rc (`fastmail-tls`) |
| `account check` | `imap.tls.provider` beside the account `tls.cert` | NOT RUN: no such account in the live rc (`fastmail-tls`) |
| config | `tls.certs` at account level | NOT RUN: needs a scratch configuration, the run being held to the live rc |
| `mailbox list` / `create` / `rename` / `delete` | `--counts`, `--json`; create of an existing name; rename and delete of a missing one; empty name; a name holding `/` | ✅ `mailbox-duplicate` exit 1; `mailbox-not-found` exit 1; the last two refused before the server |
| `envelope list` | base, `-s 2 -p 2`, `-r --has-attachment` (group members listed alone), `--json`, empty mailbox, default mailbox (`inbox`) | ✅ |
| `envelope list` | missing mailbox | ✅ `mailbox-not-found`, exit 1 (F4 fixed) |
| `envelope search` | `from:alice`, `from:@example.net`, `to:`, `with:eve`, `cc:`, `bcc:`, `subject:report`, `subject:"weekly report"`, `body:invoice`, a bare word, `tag:$X-ME-Annot-2` | ✅ (word matching, P1) |
| `envelope search` | `subject:repo`, `subject:repo*` | ✅ both empty (P1, P3) |
| `envelope search` | `is:unread`, `is:read`, `is:flagged`, `is:answered`, `is:draft` | ✅ |
| `envelope search` | `date:today`, `yesterday`, a day, a month, a year, `2026-11-20..`, `..2026-11-22`, `5h`, `10h`, `2d`, `1w`, an instant with and without offset | ✅ on the `Date:` header (P2) |
| `envelope search` | `date:5h or from:eve`, `not date:5h` | ✅ refused `search-unsupported`, exit 1 |
| `envelope search` | `not`, `-` after `--`, `or`, a group, `is:unread -date:yesterday`, `not date:2026-11-20..`, `is:flagged or is:answered` | ✅ |
| `envelope search` | `mid:` bare and in angle brackets, `mid:… or from:bob` | ✅ |
| `envelope search` | `has:attachment` alone and beside `from:` | ✅ refused `search-unsupported`, exit 1 |
| `envelope search` | `kind:event`; `from:`, `(from:alice`, `~draft`, `date:1hour`, 200 nested parentheses | ✅ `search-unsupported`; `search-invalid` (`invalid-value`, `unbalanced-parenthesis`, `reserved-form`, `invalid-date`, `limit-exceeded`), exit 1; `"~draft"` searched |
| `envelope search` | `--sort newest-first`, `oldest-first`, `title`, `sender`; `oldest-first -s 2 -p 2`; `-r --has-attachment` | ✅ |
| `envelope search` | `--sort relevance` | ✅ refused `search-unsupported` |
| `envelope search` | `-date:2d` without `--` | ✅ clap refuses it, `usage-invalid` exit 2, as MIGRATION.md says |
| `message add` | file with `-f seen -f flagged`, inline, stdin, `--json`, empty, a missing Maildir-style path holding `:2,`, no `--mailbox` | ✅ the path refused as no file and no header (HIM-5); `usage-invalid` exit 2 |
| `message add` | missing mailbox | ✅ `mailbox-not-found`, exit 1 (F4 fixed) |
| `message read` | pretty, multipart, `--raw`, `--json --raw` (`{data, mime, size}`), `--json`, unknown id, PEEK, `--seen` | ✅ `message-not-found`; PEEK by default; `--seen` sets `\Seen` |
| `message read` | `-m sent` (a role, the server naming it `Sent`) | ✅ resolved; `imap select sent` answers `NO`, exit 3 |
| `flag add/set/remove` | multi-flag, multi-id, set replaces, `junk` and `important`, unknown id beside a real one, no `-f`, `-f bogus` | ✅ item error exit 3; `usage-invalid` exit 2 |
| `message copy` / `move` | single, multi, unknown id alone and beside a real one, missing `--to`, default source (inbox, unknown UID) | ✅ item error exit 3; `usage-invalid` exit 2 |
| `message copy` / `move` | missing target, missing source | ✅ `mailbox-not-found`, exit 1 (F4 fixed) |
| `message delete` | from `B` with an unknown id beside, then from `trash` | ✅ `moved-to-trash` exit 3, then `deleted` |
| `attachment list` / `download` | `--inline`, none, `--json`, second download, `--stdout` with one id | ✅ `note (1).txt`, files 0600 |
| `attachment download` | `--stdout` without an attachment id | ❌ uncoded, exit 1 (F5) |
| `message compose` | from/to/cc/bcc/subject/body/attach/signature printed; body-file/signature-file/attach `--save`; `--send --save` | ✅ `-- ` delimiter, attachment part |
| `message send` | `--save` of a missing mailbox | ✅ `sent: true`, `saved: false`, item error `mailbox-not-found`, exit 3 (F4 fixed) |
| `message reply` | printed with `-P top -Q`, `--save -P bottom`, `--send --no-save` to self | ✅ `Re:`, `In-Reply-To`/`References`, quote; source flagged `\Answered` |
| `message forward` | `--save` | ✅ `Fwd:`, quote, attachment carried |
| `vacation get` | | ✅ refused `capability-missing`, naming `himalaya sieve` |

## Findings

### Bugs / issues

- **F4 (2026-11-18): fixed.** A missing mailbox is coded `mailbox-not-found` on `SELECT`, `APPEND`, `MOVE`, `RENAME`, `DELETE` and the send copy, the server's `NO` kept in `sources`.
- **F5: `attachment download --stdout` without an id is uncoded.** `himalaya -a fastmail attachment download 6 -m live-2026-11-26-himalaya-A --stdout` prints ``Error: `--stdout` takes exactly one attachment id``, exit 1. The command line is what is wrong, which commands.md (exit codes) and MIGRATION.md treat as `usage-invalid`, exit 2; src/unified/attachment/download.rs `stdout_part_id` bails from `execute`, after the client is built, rather than in clap. Fixed 2026-11-26 (log 2026-11-26-the-v3-report-fixes): no id or several beside `--stdout` is refused `usage-invalid`, exit 2, before the client is built (tests/early_refusal.rs).

### Provider-specific behaviour (not bugs)

- **P1: Fastmail (Cyrus) matches whole words.** `subject:repo` finds nothing, `subject:report` finds both; himalaya sends the substring `SEARCH` the RFC asks for.
- **P2: date terms read the `Date:` header**: every fixture arrived within the same minute and still split by its `Date:` on `yesterday`, `5h`, `2d` and the instants.
- **P3: a prefix term finds nothing on Fastmail.** `subject:repo*` lists nothing: the lowering reads `PimdirQueryText::text` alone (src/unified/email/search/query.rs), so the `*` is dropped before `SEARCH`, which is right for an RFC substring server but not for Fastmail's word matching. Ruled 2026-11-26 (a-prefix-or-phrase-is-never-dropped): IMAP answers a prefix by its stem as a substring, so on Fastmail it finds only the whole word (`subject:receipt*` lists `Your receipt from Fastmail`, `subject:integ*` and `subject:subscr*` nothing, rechecked on the release build). A phrase is sent as one string, which Fastmail matches as its words in any order: `subject:"test integration"` lists `io-smtp integration test`, `subject:"date billing"` the billing message; the capability detail says so.
- **P4: the listing follows arrival.** `envelope list` orders by UID, newest first: Epsilon (dated 10 days ago, added fifth) lists above Delta (3 days ago, added fourth). Use `--sort newest-first` for the `Date:` order.

### Observations

- `envelope search --help` says text matches "a substring on IMAP and JMAP" in its first paragraph, while the capability lines below it say Cyrus and Fastmail match whole words: on this provider the first sentence is wrong.
- A message composed on an account without `email` and without `--from` carries no `From:`, its `Message-ID` on the machine's hostname, as the composer spec says.
- An inline message argument such as `nope:missing.eml` opens on a valid header field name, so it is sent to the server (`NO Message contains invalid header`, exit 1) rather than refused as a path: HIM-5 holds for paths whose name before the colon breaks a field name.
- Flag symbols: `*` unseen, `!` flagged, `R` answered, `@` attachment.

## Verdict

IMAP + SMTP on Fastmail is **solid for v3**: the pimdir query language with its refusals, relative dates and instants, role resolution, the send-copy receipt with exit 3 and private downloads all pass against the live server, and F4 is fixed. One small bug: `attachment download --stdout` without an id exits 1 uncoded instead of `usage-invalid` (F5). The TLS rows were not run, their scratch accounts being absent from the live rc.
