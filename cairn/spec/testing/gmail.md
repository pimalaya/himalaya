---
cairn: spec
capability: testing
status: current
---

# Gmail REST on Google: unified-command test report

Unified commands on the Gmail REST backend. Companion: [gmail-specific.md](gmail-specific.md) (the `gmail …` raw API). Method: [provider-test-plan.md](provider-test-plan.md), inside throwaway labels.

- himalaya: release build of master 6dd75523 (the three v3 batches of 2026-11-18), default features; `--version` still prints `v2.2.1`
- account: `google` (Gmail REST API, `gmail.auth.token.command`), the pimalaya.org Workspace test mailbox, which holds a fake corpus kept untouched
- date: 2026-11-18
- fixtures: labels `himalaya-v3-live-A` (`Label_532`), `himalaya-v3-live-B` (`Label_533`) and `himalaya-v3-live-T` (`Label_534`, 120 messages for the unpaged listing), six messages added with `message add` dated relative to the run (3 hours, 8 hours, yesterday, 3 days, 10 days ago, and a `multipart/mixed` with `note.txt`). Every message and label the run made was deleted at the end; the profile counts came back to 487 messages and 419 threads.
- live suite: `cargo test --test google -- --ignored`, 11 passed (anonymous and credentialed, `gmail_writes` included).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `account check` | text, `--json` (row `tls` all `null`) | ✅ |
| `mailbox list` | `--counts` | ✅ |
| `mailbox create` / `rename` / `delete` | label lifecycle, rename by id | ✅ |
| `envelope list` | by name, by id, `-s 2 -p 2`, `--json`, empty label | ✅ |
| `envelope list -s 0` | 120 messages over two Gmail pages | ✅ all 120, `nextPage` `null` |
| `envelope search` | `today`, `yesterday`, `2 days ago`, `1 week ago`, `5 hours ago` (after, before, under `or` and `not`) | ✅ |
| `envelope search` | conjunctions under a disjunction, `from … and (… or attachment)`, `order by date asc`, `flag flagged or not flag seen`, `body`, `attachment`, `message-id`, `subject OR` | ✅ |
| `envelope search` | `flag answered`, `after 15-11-26` | ✅ refused (`search-unsupported`, `search-invalid`) |
| `message add` | `-f seen`, `-f flagged`, `--json` | ✅; `-f answered` refused `capability-missing` |
| `message read` | pretty, multipart, `--raw`, `--json`, unknown id | ✅ (`message-not-found`) |
| `flag add/set/remove` | seen, flagged, important, unknown id beside a real one | ✅ (item error, exit 3); answered refused `capability-missing` |
| `message copy` / `move` | multi-id, unknown id, missing target label | ✅; missing label uncoded (G6) |
| `attachment list` / `download` | `--json`, second download | ✅ `note (1).txt`, files 0600 |
| `message send` | `--save` of a missing label | ✅ `sent: true`, item error, exit 3; uncoded (G6) |
| `message compose --send --save` | to self | ✅ one message, labelled, its id reported |
| `message reply --send --no-save` / `forward --save` | to self | ✅ |
| rate limit | per-minute quota met during the search run | ✅ items `rate-limited`, exit 3 |

## Findings

### Bugs

- **G6: a missing label is reported without a code.** `himalaya -a google --json message move -f himalaya-v3-live-A -t himalaya-v3-live-nope <id>` prints ``{"code":null,"error":"Gmail has no label `himalaya-v3-live-nope`, by id, name or role",…}``, exit 1, and `message send --save himalaya-v3-live-nope` lists the save failure with `"code":null`, exit 3. The same class shows on IMAP, where the code table promises `mailbox-not-found` (imap-smtp-fastmail.md, F4); Gmail is not named in that row, so here it is a gap rather than a contract break, but "coded by its cause" (*Saving a sent message follows the send*) reads as `mailbox-not-found`.

### Behaviour (not bugs)

- The Gmail search reached the per-minute quota (`Units per minute per user`) after about twenty searches in a row. The per-message reads of `envelope search` then printed one `rate-limited` item error each (exit 3), and a whole-command refusal printed `{"refusal":{"status":403,"reason":"rateLimitExceeded",…,"code":"rate-limited","retryAfter":null}}`, exit 3, under `gmail request`. The same searches passed after a two-minute pause. The daily limit (COH-4) was not met.
- Gmail has no `answered` label: `message add -f answered`, `flag add -f answered` and `envelope search flag answered` are refused before any write. A reply's source is therefore not flagged on Gmail.
- A message sent to the account itself is one message carrying `INBOX` and `SENT`; `compose --send --save <label>` labels that message rather than adding a copy.
- An inserted message gains a `Received:` header from `gmailapi.google.com`, which `message read --raw` shows verbatim.
- Gmail date clauses compare the received date (`partial`); the inserted fixtures took their `Date:` as the internal date, so the relative-date cells matched as on the local backends.

## Verdict

Unified commands on Gmail are **solid**: `message add` and `envelope search` now work, the unpaged listing reads every page (HIM-3), conjunctions under `or` keep their groups (HIM-2), relative dates and instants resolve as specified, and the rate-limited code reaches both item errors and refusals. One gap: a missing label is uncoded (G6).
