---
cairn: spec
capability: testing
status: current
---

# JMAP on Fastmail: unified-command test report

Unified commands on the JMAP backend. Companion: [jmap-specific-fastmail.md](jmap-specific-fastmail.md) (the `jmap …` raw API). Method: [provider-test-plan.md](provider-test-plan.md), inside two throwaway JMAP mailboxes (`himalaya-v3-live-J1`/`-J2`).

- himalaya: release build of master c66b8629 (the three v3 batches of 2026-11-18), default features
- account: `fastmail-jmap` (`jmap.server = https://api.fastmail.com/jmap/session`, bearer token; no identity or drafts id configured)
- date: 2026-11-18
- fixtures: the six messages of the IMAP run (dated 3 hours to 10 days before it, one `multipart/mixed` with `note.txt`), imported with `message add`. Fastmail deduplicated them against the identical bytes the IMAP run had appended: each import returned the existing Email, now in both run mailboxes (J2). Everything was deleted at the end with the run mailboxes; a whole-account `jmap email query --subject v3live` finds nothing.
- live suite: `cargo test --test fastmail -- --ignored`, 2 passed (anonymous only: Fastmail issues tokens through a browser consent).

## Results

| Command | Variants tested | Result |
| --- | --- | --- |
| `account check` | text, `--json` | ✅ |
| `mailbox list` / `create` / `delete` | `--counts`, `--json` | ✅ |
| `envelope list` | by name, by id, `-s 2 -p 2`, `-r`, empty mailbox | ✅ |
| `envelope search` | `today`, `yesterday`, `2 days ago`, `1 week ago`, `5 hours ago` after and before, on the root `and` | ✅ sent date re-checked |
| `envelope search` | a date under `or` or `not` | ⚠️ reads the received date (J4) |
| `envelope search` | `from … and (… or attachment)`, `order by date asc`, `order by subject desc`, flags, `body`, `attachment`, `subject rep` | ✅ (word matching) |
| `envelope search` | `message-id` | ✅ refused `search-unsupported` |
| `message add` | `-f`, `--json` | ✅ |
| `message read` | pretty, multipart, `--raw` (and `--json` Base64), unknown id | ✅ (`message-not-found`) |
| `flag add/set/remove` | multi-flag, set replaces, unknown id beside a real one | ✅ item error, exit 3 |
| `message copy` / `move` | multi-id, unknown id | ✅ item error, exit 3 |
| `message move` | missing target | ⚠️ uncoded (J5) |
| `attachment list` / `download` | `--json`, second download | ✅ `note (1).txt` |
| `message send` | `--save` of a missing mailbox | ✅ `sent: true`, item error, exit 3; uncoded (J5) |
| `message compose --send --save` | to self | ✅ |
| `message reply --send --no-save` | to self | ✅ (J6) |
| `message forward --save` | | ✅ |
| `vacation get` | | ✅ `Enabled: no` |

## Findings

### Bugs

- **J5: a missing mailbox is reported without a code.** `himalaya -a fastmail-jmap --json message move -f himalaya-v3-live-J1 -t himalaya-v3-live-nope <id>` prints ``{"code":null,"error":"Mailbox `himalaya-v3-live-nope` not found",…}``, exit 1, and the send copy to a missing mailbox lists `"code":null`. Same class as IMAP F4; JMAP's unified commands are not named in the `mailbox-not-found` row, so this is a gap rather than a broken promise.

### Behaviour / config (not bugs)

- **J4: under `or` or `not`, a date clause reads the received date.** `envelope search -m himalaya-v3-live-J1 -- after 5 hours ago or from eve` listed all six messages and `not after 5 hours ago` (or `not after yesterday`) none, because every fixture was received minutes before. search.md specifies it (the client-side residual cannot hold across a disjunction or a negation, the JMAP date clauses declared `partial`), but `envelope search --help` says "A date clause reads the `Date:` header": the help could name the exception, as it does for IMAP's instants.
- **J6: a JMAP send always lands in Sent.** The submission's `onSuccessUpdateEmail` moves the message to the sent role, so `reply --send --no-save` and the send whose `--save` failed both left a copy in Sent while their receipts said `saved: false`. backends.md specifies the move; the receipt does not count it as a save.
- **J2 (kept): a bogus id is an item error** coded `message-not-found`, never a silent no-op.
- Sending works from the minimal account (identity and drafts mailbox discovered, J3).
- A raw message without `Date:` sent through `message send` keeps none: the Sent copy lists no date.

## Verdict

Unified commands on JMAP are **solid**: relative dates on the root `and` (re-checked on the sent date, east-of-UTC days included), refusals with codes, the send-copy receipt with exit 3 and the blob downloads all pass. One gap: a missing mailbox is uncoded (J5); J4 and J6 are specified behaviours the help and the receipt could make plainer.
