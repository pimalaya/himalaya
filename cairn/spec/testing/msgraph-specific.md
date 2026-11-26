---
cairn: spec
capability: testing
status: current
---

# Microsoft Graph *specific* API on Outlook: test report

Companion to [msgraph.md](msgraph.md) (unified commands). Exercises `himalaya msgraph …`: the raw Microsoft Graph mail surface.

- himalaya: release build of master 944ba0ad plus the uncommitted v3 prep, `--all-features`, `--version` v3.0.0
- account: `microsoft` (Microsoft Graph, the test tenant)
- date: 2026-11-26
- method: the `msgraph` subcommands inside the run's folders, per [provider-test-plan.md](provider-test-plan.md): `live-2026-11-26-himalaya-A` and `-B` (the unified run's), plus `live-2026-11-26-himalaya-C` (renamed `-C2`) for the folder lifecycle; messages with subjects starting `live-2026-11-26-himalaya`. Every folder and message created was deleted, the sends' Inbox and Sent Items copies included: 12 folders before and after, every count identical, and no marker left in Inbox, Sent Items, Drafts or Deleted Items.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `profile get` | text, `--json` | ✅ Graph's `/me` payload as sent |
| `request` | `get me/messages/999`, text and `--json` | ✅ `{refusal: {status: 400, reason: ErrorInvalidIdMalformed, …}}`, exit 3 |
| `request` | off-origin URL, text and `--json` | ✅ refused `request-origin` before the session, exit 1, as the commands spec says |
| `request` | `get me/mailFolders/inbox`, `get me/outlook/masterCategories`, method `put` | ✅ `{response}`; `put` refused `usage-invalid`, exit 2 |
| `mail-folders create/rename/get/delete` | lifecycle, create of an existing name, `get` after delete | ✅ 409 `ErrorFolderExists` and 404 `ErrorItemNotFound` printed as refusals, exit 3 |
| `mail-folders list` | `--top 2`, `--page-token` of its next link | ✅ Graph's `@odata.nextLink` as `nextPage`, followed as is |
| `mail-folders move` / `copy` | `-C2` into `A`, a copy into `B` | ✅ `{id}`; the copy deleted with `mail-folders delete` |
| `mail-folders child-folders` | empty folder, a folder holding one | ✅ `{folders: [], nextPage: null}`, then the child |
| `messages create` | raw MIME into `drafts` (the six fixtures and one draft) | ✅ `{id}` |
| `messages list` | `--folder --top 2 --select subject`, `--page-token` of its next link | ✅ next link as `nextPage`, followed as is |
| `messages list` | `--page-token` beside `--top` | ✅ refused `usage-invalid`, exit 2 |
| `messages list` | `--search subject:quarterly`, `--filter 'isRead eq false'`, `--filter 'isRead eq true' --count` | ✅ `@odata.count: 4` in the payload |
| `messages list` | `--filter 'hasAttachments eq true'`, then with `--orderby 'hasAttachments desc'` | ✅ Graph refuses the first `InefficientFilter` (400), exit 3; the second answers |
| `messages get` | parsed, `--raw` | ✅ Graph's resource whole; raw MIME |
| `messages update` | `--unread --importance low --category live-2026-11-26-himalaya`, back with `--read --importance normal --no-categories`, no change | ✅ `{id}`, text names the id; no change refused `usage-invalid`, exit 2; master categories untouched |
| `messages copy` | `<id> <folder>` | ✅ `{id}`, the copy's id in `B`, read at once by `messages get` |
| `messages move` | malformed id, no destination | ✅ refusal 400, exit 3; `usage-invalid`, exit 2 |
| `messages send` | raw MIME to self | ✅ `{id: null}`, delivered and filed in Sent Items |
| `messages send-draft` | a created draft, malformed id | ✅ `{id}` (the draft's), delivered; refusal 400, exit 3 |
| `messages delete` | the raw copy in `B` | ✅ `{id}`; `messages get` then 404, nothing in Deleted Items |
| `attachments list` / `get` | `note.txt`, printed and `-o <path>` | ✅ bytes as sent; `-o` prints `{path, size}` |
| `attachments create` / `delete` | a file on a moved draft (`-n -t`), a missing local file | ✅ `{id}`, listed, deleted; the missing file uncoded, exit 1 (observation) |

## Findings

### Behaviour / usability

- **M3 (2026-11-18) not reproduced:** Graph's copy is documented as asynchronous, but the id `messages copy` returned was the copy's in `B`, readable at once.
- `messages list --filter 'hasAttachments eq true'` is refused by Graph (`{"refusal":{"status":400,"reason":"InefficientFilter",…}}`, exit 3) because the default `--orderby receivedDateTime desc` does not open with the filtered property; `--orderby 'hasAttachments desc'` answers. Graph's rule, not Himalaya's; `isRead` filters pass.
- `attachments get -o` writes the file under the umask (0644 here), while the unified `attachment download` writes 0600; the commands spec asks 0600 of the unified download only.
- `attachments list` gives `size: 285` for the 19-byte `note.txt`, Graph's size of the attachment item, not of its content.
- `attachments create <id> nope.txt` prints `{"code":null,"error":"No such file or directory (os error 2)",…}`, exit 1, without naming the path.
- `messages send` answers `{id: null}`, `sendMail` returning no id; `send-draft` answers the draft's id, which the sent copy no longer carries; the delivered copy is found by subject.
- The specific commands take folder ids or Graph's well-known names (`drafts`, `deleteditems`), not display names; the unified commands resolve names (msgraph.md).
- A deleted mail folder did not show under Deleted Items' child folders afterwards (`child-folders deleteditems --hidden` lists none), and `messages delete` leaves nothing in Deleted Items.
- A malformed id is printed as Graph's refusal with `code: null`; the unified commands code the same answer `message-not-found`.

## Verdict

The raw Graph surface **works end-to-end**: refusals print the refusal object with exit 3, next links page as given and refuse query options beside them, an off-origin URL is refused `request-origin` before the token leaves, and the arguments Graph would refuse are refused before the session. No bug found on this surface; the notes are Graph rules and small usability points (an uncoded missing file, `-o` under the umask).
