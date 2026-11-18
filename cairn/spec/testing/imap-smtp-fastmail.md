---
cairn: spec
capability: testing
status: current
---

# IMAP + SMTP on Fastmail: unified-command test report

- himalaya: release build of master c66b8629 (the three v3 batches of 2026-11-18), default features; `--version` still prints `v2.2.1`
- account: `fastmail` (`imaps://imap.fastmail.com`, `smtps://smtp.fastmail.com`, SASL PLAIN), plus two scratch copies of it: `fastmail-tls` (an account-level `tls.cert`) and the same with `imap.tls.provider` overriding it
- date: 2026-11-18
- method: every unified command and flag variant, inside throwaway mailboxes `himalaya-v3-live-A`, `-B` and `-T` (260 threaded messages for the chunked thread fetch), per [provider-test-plan.md](provider-test-plan.md). Fixtures: six messages dated 3 hours, 8 hours, yesterday, 3 days and 10 days before the run, one `multipart/mixed` with `note.txt` and a `To:` group. Every delivered copy was moved into `A`, and the mailboxes were deleted at the end; Inbox, Sent, Drafts and Trash came back to their starting counts.

## Results

| Command | Variants tested | Result |
| --- | --- | --- |
| `account check` | text, `--json` | ✅ `tls` row per backend |
| `account check` | account `tls.cert` | ✅ both IMAP and SMTP name it; a missing file fails both, exit 3 |
| `account check` | `imap.tls.provider` beside the account `tls.cert` | ✅ IMAP shows `rustls, cert …`, SMTP `cert …` (field-by-field override) |
| config | `tls.certs` at account level | ✅ refused by serde, naming the line and the known keys |
| `mailbox list` / `create` / `delete` | `--counts`, `--json` | ✅ |
| `envelope list` | base, `-s 2 -p 2`, `-r` (group members listed alone), `--json`, empty mailbox, default mailbox | ✅ |
| `envelope search` | `today`, `yesterday`, `2 days ago`, `1 week ago`, `15-11-2026`, `5 hours ago` after and before, an RFC 3339 instant in UTC | ✅ |
| `envelope search` | an instant under `or` or `not` | ✅ refused `search-unsupported` |
| `envelope search` | `date 1 hour ago`, `after 15-11-26`, 200 nested parentheses | ✅ refused `search-invalid` |
| `envelope search` | `order by date asc`, `order by subject desc`, `flag … or flag …`, `not flag seen`, `body`, `message-id`, `subject rep` | ✅ (word matching, P1) |
| `envelope search` | `attachment` | ✅ refused `search-unsupported` |
| `message add` | file, stdin, `-f` multi, `--json`, empty, a missing path holding a colon | ✅; the colon path is refused as no file and no header (HIM-5) |
| `message add` | missing mailbox | ⚠️ uncoded (F4) |
| `message read` | pretty, multipart, `--raw`, `--json`, unknown id, `--seen` | ✅ PEEK by default; `--seen` sets `\Seen` |
| `message read` | `-m sent` (a role, the server naming it `Sent`) | ✅ resolved (HIM-7); `imap select sent` answers `NO` |
| `flag add/set/remove` | multi-flag, multi-id, set replaces, unknown id beside a real one, errors | ✅ item error, exit 3 |
| `message copy` / `move` | single, multi, unknown id, missing `--to` | ✅ counts from `COPYUID`, item error exit 3 |
| `message move` | missing target | ⚠️ uncoded (F4) |
| `message delete` | from `A`, then from `trash` | ✅ `moved-to-trash`, then `deleted` |
| `attachment list` / `download` | `-i`, none, `--json`, second download | ✅ `note (1).txt`, files 0600 |
| `message compose` | from/to/cc/bcc/subject/body/body-file/attach/signature/signature-file/save/send | ✅ |
| `message send` | `--save` of a missing mailbox | ✅ `sent: true`, `saved: false`, item error, exit 3; uncoded (F4) |
| `message reply` | `--save`, `--send --no-save` | ✅ `Re:`, threading, quote; source flagged `\Answered` |
| `message forward` | `--save` | ✅ |

## Findings

### Bugs / issues

- **F4: a missing IMAP mailbox is not coded `mailbox-not-found`.** The code table of commands.md lists IMAP under `mailbox-not-found`, and the send-copy scenario prints that code, but the reads and writes that meet the server's `NO` print `code: null`:
  - `himalaya -a fastmail --json envelope list -m himalaya-v3-live-nope` prints `{"code":null,"error":"IMAP SELECT failed: NO Mailbox does not exist",…}`, exit 1;
  - `himalaya -a fastmail --json message add -m himalaya-v3-live-nope -- alpha.eml` prints `{"code":null,"error":"IMAP APPEND failed: NO Mailbox does not exist",…}`, exit 1;
  - `himalaya -a fastmail --json message move -f himalaya-v3-live-A -t himalaya-v3-live-nope 5` prints `{"code":null,"error":"IMAP MOVE failed: NO Mailbox does not exist",…}`, exit 1;
  - `himalaya -a fastmail --json message send --save himalaya-v3-live-nope -- send1.eml` prints `{"id":null,"saved":false,"sent":true,"queued":false,"errors":[{"id":null,"code":null,"error":"Message sent, but saving a copy to himalaya-v3-live-nope failed","sources":["IMAP APPEND failed: NO Mailbox does not exist"]}]}`, exit 3.
  src/imap/backend.rs `explain` codes a missing mailbox for the writes it wraps (a create under a parent); `SELECT`, `APPEND` and `MOVE` do not pass through it. Fastmail answers a plain `NO` with no `[NONEXISTENT]` code.

### Provider-specific behaviour (not bugs)

- **P1: Fastmail (Cyrus) matches whole words.** `subject rep` finds nothing, `subject report` finds both; himalaya sends the substring `SEARCH` the RFC asks for.
- **P2: date clauses read the `Date:` header**, as the help says: every fixture arrived within the same minute and still split by its `Date:` on `yesterday`, `2 days ago` and the instants.
- **P4: the listing follows arrival.** `envelope list` orders by UID, newest first: Epsilon (dated 10 days ago, added fifth) lists above Delta (3 days ago, added fourth). Use `envelope search order by date desc` for the `Date:` order.

### Observations

- An account-level `tls.cert` that does not exist fails every transport with `No such file or directory (os error 2)` and `code: null`: the message does not name the path, which the row's `tls.cert` does.
- `after 15-11-26` is refused, but its diagnostic reads `found -11-26, expected something else or space after the number` rather than naming the four-digit year.
- A reply composed on an account without `email` and without `--from` carries no `From:`; the composer's `Message-ID` then sits on the machine's hostname, as the composer spec says.
- Flag table symbols: `*` unseen, `!` flagged, `R` answered, `@` attachment (JMAP, Maildir).

## Verdict

IMAP + SMTP on Fastmail is **solid for v3**: relative dates and instants, the bounded query, the account-level TLS policy and `account check` rows, role resolution on reads, the send-copy receipt with exit 3 and private downloads all pass against the live server. One bug: a missing mailbox is uncoded on IMAP (F4), against the code table.
