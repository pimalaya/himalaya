# Microsoft Graph on Outlook — shared-command test report

Shared cross-protocol commands on the Microsoft Graph backend.
Companion: [msgraph-specific.md](msgraph-specific.md) (the `msgraph …`
raw API). Method: [provider-test-plan.md](provider-test-plan.md),
inside a throwaway Graph mail folder.

- himalaya: `v2.0.0-alpha.1 +msgraph +rustls-ring` (working tree)
- account: `msgraph` (Microsoft Graph REST API, OAuth via `ortie token
  show -a msgraph`; a bearer token scoped to `https://graph.microsoft.com`
  mail permissions — `Mail.ReadWrite`, `Mail.Send`)
- date: 2026-07-18
- account locale: French (folder display names are localised, e.g.
  `Boîte de réception` for the inbox), which makes the name→id issue easy
  to see: Graph accepts English *well-known* names (`inbox`, `archive`,
  …) in the path but not the localised display name.
- fixtures: two throwaway folders `Himalaya-Test` / `Himalaya-Test-2`,
  populated with two drafts created straight into the folder (one
  `multipart/mixed` with a small `note.txt` attachment). Both folders and
  every sent-test message were deleted afterwards; Deleted Items checked
  clean.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `mailbox list` | base (folders + counts) | ✅ |
| `envelope list` | by folder **name**, by id, well-known name, `-s`, `--json` | ✅ (name resolves, M1 fixed) |
| `envelope search` | — | ⚪ Graph has no shared search (bails cleanly) |
| `message add` | — | ⚪ Graph has no append (bails cleanly) |
| `message read` | pretty, `--json` (folder ignored) | ✅ |
| `flag add/set/remove` | seen/flagged/importance → Graph scalar fields | ✅ |
| `message copy` / `move` | `--from`/`--to` by name, F2 counts | ✅ |
| `attachment list` / `download` | list, `-d` | ✅ |
| `message send` | real send to self | ✅ delivered (Inbox + Sent) |

## Findings

### Bugs

- **M1 — shared `-m <folder-name>` now resolves to a folder id. FIXED.**
  `envelope list -m "Boîte de réception"` (a localised display name) used
  to fail `HTTP 400 (ErrorInvalidIdMalformed): Id is malformed`; only the
  opaque id or an English well-known name (`inbox`) worked. Graph mail
  folders are opaque-id + display-name like JMAP mailboxes and Gmail
  labels, so `MsgraphClient` now gains a cached `resolve_mailbox_id`
  (name → id via a `mailFolders` listing, id passthrough, unknown value
  handed back so Graph well-known names still reach the API), wired into
  `EmailClient::resolve_mailbox_id` next to the JMAP and Gmail arms.
  Verified: `envelope list`/`flag`/`copy`/`move` now address folders by
  localised name, by opaque id, or by well-known name.

### Behaviour (not bugs)

- `message read` works regardless of `-m` because the Graph backend
  fetches by the globally-unique message id and ignores the folder (like
  JMAP `get_message` and the Gmail backend).
- `message add` and `envelope search` bail with clear messages
  (`Microsoft Graph does not support adding messages` / `… the shared
  envelope search`) — Graph has no MIME append and no server-side
  shared-query search (its own `$search`/`$filter` live on the specific
  `msgraph message list`).
- `flag`/`copy`/`move` map onto Graph's model: `seen` = `isRead`,
  `flagged` = the follow-up flag, `important` = `importance: high`,
  non-IANA flags = `categories`; copy = `POST /messages/{id}/copy`, move
  = `POST /messages/{id}/move`. The compact text FLAGS column has no
  glyph for `\Draft`/`\Seen`, so freshly-created read drafts show an
  empty column even though `--json` lists the flags correctly.

## Verdict

Shared commands on Microsoft Graph are **solid** for the supported
subset (Graph legitimately lacks shared `add`/`search`):
list/read/flag/copy/move/attachment/send all work, now addressing
folders by **name, id, or well-known name** (M1 fixed). No functional
blocker.
