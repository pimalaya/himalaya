---
cairn: spec
capability: testing
status: current
---

# Microsoft Graph on Outlook: unified-command test report

Unified commands on the Microsoft Graph backend. Companion: [msgraph-specific.md](msgraph-specific.md) (the `msgraph …` raw API). Method: [provider-test-plan.md](provider-test-plan.md), inside throwaway mail folders.

- himalaya: release build of master c66b8629 (the three v3 batches of 2026-11-18), default features
- account: `microsoft` (Microsoft Graph, `msgraph.auth.token.command`), the Exchange Online test tenant, which holds a fake corpus kept untouched; its folders list under their English names (`Inbox`, `Sent Items`, `Deleted Items`) although the user's preferred language is French
- date: 2026-11-18
- fixtures: folders `himalaya-v3-live-A` and `himalaya-v3-live-B`; Graph has no MIME append, so six drafts were made with `msgraph messages create` (the same relative dates as the other reports) and moved into `A` with `message move -f drafts`. Both folders were deleted at the end, after the run's Inbox and Sent Items copies were moved into `A`; Inbox, Sent Items, Drafts and Deleted Items hold no run message.
- live suite: `cargo test --test microsoft -- --ignored`, 11 passed (anonymous and credentialed, `graph_writes` included).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `account check` | text, `--json` | ✅ |
| `mailbox list` / `create` / `delete` | `--counts` | ✅ |
| `envelope list` | by name, `-s 2 -p 2`, `-r`, empty folder, role `inbox` | ✅ |
| `envelope search` | `yesterday`, `2 days ago`, `1 week ago`, `from … and (… or attachment)`, `order by date asc`, `body`, `attachment`, `message-id`, `not from`, `from … or from …`, `subject OR` | ✅ |
| `envelope search` | `after 5 hours ago`, `flag flagged` | ✅ refused `search-unsupported` (KQL compares days, has no flag) |
| `message add` | | ✅ refused `capability-missing` (Graph creates drafts only) |
| `message read` | pretty, `--raw`, malformed id | ✅ (`message-not-found`, Graph's 400 in `sources`) |
| `flag add/set/remove` | seen, flagged, important, malformed id beside a real one | ✅ item error exit 3; `answered` refused `capability-missing` |
| `message copy` / `move` | multi-id, malformed id, missing target | ✅; missing target uncoded (M4) |
| `attachment list` / `download` | `--json` | ✅ file 0600 |
| `message send` | `--no-save` | ✅ refused `capability-missing` before connecting |
| `message send` | `--save` of a missing folder | ✅ `saved: true`, `sent: true` (Graph files in Sent Items, `--save` ignored) |
| `message reply --send` | to self | ✅ |

## Findings

### Bugs

- **M4: a missing folder is reported without a code.** `himalaya -a microsoft --json message move -f himalaya-v3-live-A -t himalaya-v3-live-nope <id>` prints ``{"code":null,"error":"Mailbox `himalaya-v3-live-nope` not found",…}``, exit 1. Same class as Gmail G6 and IMAP F4; Graph is not in the `mailbox-not-found` row of the code table, so it is a gap there rather than a broken promise.

### Behaviour (not bugs)

- Graph files every sent message in Sent Items itself: `--save` is ignored on a send and the receipt says `saved: true` with a `null` id; `--no-save` is refused before connecting. The send-copy failure path cannot be provoked on Graph.
- Instants are refused (`after 5 hours ago`): KQL `sent` compares days, as the search spec says.
- Moved drafts keep `\Draft` (Graph's `isDraft` is read-only), so every fixture listed `\Draft`.
- `envelope list -r` shows the account owner's display name for its own address, as Graph's directory resolves it.

## Verdict

Unified commands on Graph are **solid** for the subset Graph serves: list, search (days, KQL text, `attachment`, `message-id`), read, flags, copy, move, attachments and send all pass, and the unsupported cells are refused with a code before any write. One gap: a missing folder is uncoded (M4).
