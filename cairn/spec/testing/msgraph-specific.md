---
cairn: spec
capability: testing
status: current
---

# Microsoft Graph *specific* API on Outlook: test report

Companion to [msgraph.md](msgraph.md) (unified commands). Exercises `himalaya msgraph …`: the raw Microsoft Graph mail surface.

- himalaya: release build of master c66b8629, default features
- account: `microsoft` (Microsoft Graph, the test tenant)
- date: 2026-11-18
- method: the `msgraph` subcommands inside the run's folders, per [provider-test-plan.md](provider-test-plan.md). Every folder and message created was deleted.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `profile get` | `--json` | ✅ Graph's `/me` payload as sent |
| `request` | `get` of a malformed id, text and `--json` | ✅ `{refusal: {status: 400, reason: ErrorInvalidIdMalformed, …}}`, exit 3 |
| `request` | off-origin URL | ✅ refused `usage-invalid`, exit 2 |
| `mail-folders create/rename/get/delete` | lifecycle | ✅ |
| `mail-folders child-folders` | empty folder | ✅ `{folders: [], nextPage: null}` |
| `messages create` | raw MIME (into Drafts) | ✅ |
| `messages list` | `--folder --top 2`, `--page-token` of its next link | ✅ Graph's `@odata.nextLink` as `nextPage`, followed as is |
| `messages list` | `--page-token` beside `--top` | ✅ refused `usage-invalid`, exit 2 |
| `messages get` | parsed, `--raw` | ✅ Graph's resource whole; raw MIME |
| `messages update` | `--unread --importance low --category`, no change | ✅; no change refused `usage-invalid`, exit 2 |
| `messages copy` | `<id> <folder>` | ✅ |
| `messages move` | malformed id | ✅ refusal 400, exit 3 |
| `messages send` | raw MIME to self | ✅ `{id: null}` |
| `attachments list` / `get` | `note.txt` | ✅ |
| `attachments create` / `delete` | a file on a moved draft | ✅ |

## Findings

### Behaviour / usability

- **M3 (kept): Graph's `copy` is asynchronous.** Not re-provoked in this run; the copy returned an id used by no later command.
- The specific commands take folder ids or Graph's well-known names, not display names; the unified commands resolve names (msgraph.md).
- A deleted mail folder did not show under Deleted Items' child folders afterwards.
- `messages send` answers `{id: null}`, `sendMail` returning no id; the delivered copy is found by subject.

## Verdict

The raw Graph surface **works end-to-end**: refusals print the refusal object with exit 3, the next link pages as given and refuses query options beside it, and the arguments Graph would refuse are refused before the session. No bug found on this surface.
