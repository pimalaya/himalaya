---
cairn: spec
capability: testing
status: current
---

# Gmail *specific* API on Google: test report

Companion to [gmail.md](gmail.md) (unified commands). Exercises `himalaya gmail …`: the raw Gmail REST v1 surface.

- himalaya: release build of master c66b8629, default features
- account: `google` (Gmail REST, the pimalaya.org Workspace test mailbox)
- date: 2026-11-18
- method: the `gmail` subcommands inside the run's labels, per [provider-test-plan.md](provider-test-plan.md). Account-wide setting writes were not exercised (reads only). Everything created was deleted (`messages batch-delete` of the 133 run messages, then the labels).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `profile get` | `--json` | ✅ Gmail's payload as sent |
| `request` | `get` of a missing label, text and `--json` | ✅ `{refusal: {status: 404, reason: notFound, …}}`, exit 3 |
| `request` | off-origin URL, method `head` | ✅ refused `usage-invalid`, exit 2, before the session |
| `labels create/get/update/delete` | lifecycle | ✅ |
| `labels update` and `mailbox rename` | on a label coloured through `request patch` | ✅ colour and `labelListVisibility` kept |
| `labels get` / `messages get` | unknown id | ✅ refusal 404 / 400, exit 3 |
| `messages list` | `--label`, `-q`, 120 messages | ✅ `nextPage` is Gmail's `nextPageToken` |
| `messages get` | `--format metadata --header Subject`, `raw`, `full`, `minimal` | ✅ raw payload |
| `messages insert` / `import` | `--label` | ✅ `{id, …}` |
| `messages modify` | `--add-label`/`--remove-label`, unknown id | ✅; unknown id refusal 404, exit 3 |
| `messages trash` / `untrash` | | ✅ |
| `messages send` | raw message to self | ✅ `{id}` |
| `messages batch-delete` | 133 ids | ✅ (cleanup) |
| `drafts create/update/get/list/delete` | lifecycle, `get` after delete | ✅; refusal 404 after delete |
| `threads get` / `list` | `--label` | ✅ |
| `history list` | recent id, `--start-history-id 1` | ✅; the stale id refused 404, exit 3 |
| `settings vacation/imap/pop/language/auto-forwarding get` | | ✅ |
| `settings filters/forwarding-addresses/send-as list` | | ✅ (empty lists parse) |
| `settings delegates list` | | ✅ refusal 403 `forbidden`, `permission-denied`, exit 3 |
| `attachments get` | | ⚪ not run: the fixture attachment is inlined and has no `attachmentId` (G2) |

## Findings

### Behaviour / usability

- **G2 (kept): `gmail attachments get` needs a stored attachment.** Small attachments are inlined in `body.data` with no `attachmentId`; the unified `attachment download` reads both kinds and is the path for them.
- **G5 (kept): settings writes need the settings scopes.** The reads pass with `https://mail.google.com/`; no write was attempted.
- The single reads (`profile get`, `messages get`, `threads get`, `drafts get`, `labels get`, the settings reads) print Gmail's payload whole (single-reads-print-raw), so a key himalaya does not model, such as `labelListVisibility`, survives.
- `settings delegates list` is refused for a delegated service-account token; the refusal now carries the code `permission-denied` beside Gmail's reason.

## Verdict

The raw Gmail surface **works end-to-end**: every refused request prints the refusal object and exits 3, off-origin URLs and bad methods are refused before the session, and a label rename keeps its colour (HIM-20). No bug found on this surface.
