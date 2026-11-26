---
cairn: spec
capability: testing
status: current
---

# Gmail *specific* API on Google: test report

Companion to [gmail.md](gmail.md) (unified commands). Exercises `himalaya gmail …`: the raw Gmail REST v1 surface.

- himalaya: release build of master 944ba0ad plus the uncommitted v3 prep, `--all-features`; `--version` prints `v3.0.0`
- account: `google` (Gmail REST, the pimalaya.org Workspace test mailbox)
- date: 2026-11-26
- method: the `gmail` subcommands inside the run's labels (`live-2026-11-26-himalaya-A`, `-B`, `-T`, and `live-2026-11-26-himalaya-G`, `Label_588`, for the label lifecycle), per [provider-test-plan.md](provider-test-plan.md); messages with subjects starting `live-2026-11-26-himalaya`, sends to the account's own address only. Account-wide setting writes were not run (reads only). Everything created was deleted (`messages batch-delete` of the 133 run messages, then the labels); the profile counts were 502 messages and 434 threads before and after.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `profile get` | text, `--json` | ✅ Gmail's payload as sent |
| `request` | `get` of a missing label, text and `--json` | ✅ `{refusal: {status: 404, reason: notFound, …}}`, exit 3 |
| `request` | `get` of a label, `patch` of a colour | ✅ `{response}` as Gmail sent it |
| `request` | off-origin URL | ✅ refused `request-origin`, exit 1, before the session |
| `request` | method `head` | ✅ refused `usage-invalid`, exit 2 |
| `labels create/get/update/delete` | lifecycle, `get` after delete, create of an existing name | ✅ refusals 404 and 409, exit 3 |
| `labels update` and `mailbox rename` | on a label coloured through `request patch` | ✅ colour and `labelListVisibility` kept |
| `labels list` | `--json` | ✅ |
| `labels get` / `messages get` | unknown id, malformed id | ✅ refusal 404 / 400, exit 3 |
| `messages list` | `--label` over 120 messages `-s 100`, `-q`, text | ✅ `nextPage` is Gmail's `nextPageToken` |
| `messages get` | `--format metadata --header Subject`, `raw`, `full`, `minimal` | ✅ payload whole; `raw` is the byte writer, `{data, size, mime}` under `--json`, the bytes in text |
| `messages insert` / `import` | `--label` | ✅ `{id}` |
| `messages modify` | `--add-label`/`--remove-label`, unknown id | ✅; unknown id refusal 404, exit 3 |
| `messages trash` / `untrash` | | ✅ `TRASH` added then removed |
| `messages batch-modify` | two ids | ✅ `{ids}` |
| `messages send` | raw message to self | ✅ `{id}` |
| `messages delete` | then `get` | ✅; refusal 404 after |
| `messages batch-delete` | 133 ids | ✅ (cleanup) |
| `drafts create/update/get/list/delete` | lifecycle, `get` after delete | ✅; refusal 404 after delete |
| `drafts send` | to self | ✅ `{id}` |
| `threads list` / `get` | `--label` | ✅ |
| `threads modify/trash/untrash/delete` | one-message thread, `get` after delete | ✅; refusal 404 after delete |
| `history list` | recent id with `-s 5`, `--start-history-id 1` | ✅; the stale id refused 404, exit 3 |
| `attachments get` | stdout, `-o`, `--json` without `-o`, bogus id | ✅ `{path, size}`, `{data, size, mime}`; bogus id refusal 400, exit 3 |
| `settings vacation/imap/pop/language/auto-forwarding get` | | ✅ |
| `settings filters/forwarding-addresses/send-as list` | | ✅ (empty lists parse) |
| `settings delegates list` | | ✅ refusal 403 `forbidden`, `permission-denied`, exit 3 |
| settings writes (`vacation` update, `filters` create, `send-as` signature) | | NOT RUN: the rc's Google token lacks the `https://www.googleapis.com/auth/gmail.settings.basic` scope (owner to add it and re-consent) |

## Findings

### Behaviour / usability

- **G2 (2026-11-18) no longer reproduces:** the 30-byte `note.txt` (`Content-Disposition: attachment`) carried an `attachmentId` in `messages get --format full`, and `attachments get` read it. A part Gmail inlines in `body.data` still has none; the unified `attachment download` reads both kinds.
- **G5 (kept): settings writes need the settings scope.** The reads pass with `https://mail.google.com/`; no write was attempted this run.
- `attachments get -o` writes its file at the umask's mode (0644 here) and overwrites an existing file, where the unified `attachment download` writes 0600 and suffixes a taken name. commands.md binds the 0600 rule to `attachment download` alone, so this is an inconsistency to weigh, not a contract break.
- An off-origin `request` URL is now coded `request-origin`, exit 1 (it was `usage-invalid`, exit 2, on 2026-11-18), as commands.md now specifies.
- The single reads (`profile get`, `messages get`, `threads get`, `drafts get`, `labels get`, the settings reads) print Gmail's payload whole (single-reads-print-raw), so a key himalaya does not model, such as `labelListVisibility`, survives; `messages get --format raw` and `attachments get` go through the byte writer instead.
- The protocol refusals carry `code: null` for 404, 400 and 409 (the raw namespace keeps the server's refusal as it is); `settings delegates list` is refused for a delegated service-account token and coded `permission-denied`.

## Verdict

The raw Gmail surface **works end-to-end** on v3: every refused request prints the refusal object and exits 3, off-origin URLs and bad methods are refused before the session, a label rename keeps its colour, drafts and threads go through their whole lifecycle, and `attachments get` now runs. No bug found on this surface; the settings writes wait on the settings scope.
