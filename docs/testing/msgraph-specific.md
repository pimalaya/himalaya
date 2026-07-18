# Microsoft Graph *specific* API on Outlook — test report

Companion to [msgraph.md](msgraph.md) (shared commands). Exercises
`himalaya msgraph …` — the raw Microsoft Graph mail surface (profile,
mail folders, messages, attachments).

- himalaya: `v2.0.0-alpha.1 +msgraph +rustls-ring` (working tree)
- account: `msgraph` (Microsoft Graph REST, OAuth via ortie)
- date: 2026-07-18
- method: every `msgraph` subcommand, inside two throwaway folders, per
  [provider-test-plan.md](provider-test-plan.md). Every folder and
  sent-test message was deleted afterwards; Deleted Items checked clean.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `profile get` | text + `--json` | ✅ (`--json` structured, M2 fixed) |
| `mail-folder list` | text + `--json` | ✅ |
| `mail-folder get` | by id, `--json` | ✅ |
| `mail-folder create` | `<name>` | ✅ |
| `mail-folder rename` | `<id> <name>` | ✅ |
| `mail-folder delete` | `<id>` (recursive) | ✅ (used for cleanup) |
| `message create` | `--folder <id>`, raw MIME | ✅ draft placed in folder |
| `message list` | `--folder`, `--top`/`--skip` | ✅ |
| `message get` | parsed + `--raw` | ✅ |
| `message update` | `--read`/`--unread`, `--importance`, `--category` | ✅ |
| `message send` | raw MIME | ✅ delivered |
| `message copy` | `<id> <dest>` | ✅ |
| `message move` | `<id> <dest>` | ✅ on a settled id; ⚠️ 405 on a just-copied id (M3) |
| `message delete` | permanent | ✅ (used for cleanup) |
| `attachment list` | `<msg>` | ✅ |
| `attachment get` | `<msg> <att-id> -o` | ✅ |

Not separately exercised here: `mail-folder child-folders`, `copy`,
`move` (folder-level), and `attachment create`/`delete` — same request
shapes as the verified verbs.

## Findings

### Behaviour / usability

- **M2 — `msgraph profile get --json` is now structured. FIXED.** It
  used to emit `{"message":"Id: …\n…"}` because the command formatted a
  plain `String` into a `Message` (which serializes as `{"message":
  …}`) — the same shape Gmail's `profile get` had before its fix. A
  dedicated `MsgraphProfileOutput { id, display-name, mail,
  user-principal-name }` (Display + Serialize) replaces it, so `--json`
  now emits
  `{"id":"…","display-name":"…","mail":"…","user-principal-name":"…"}`
  and the text output is unchanged.

- **M3 — moving a *just-copied* message can 405.** `msgraph message move
  <id> <dest>` on the id returned directly by `msgraph message copy`
  fails `HTTP 405 (ErrorInvalidRequest): The OData request is not
  supported`. The command is byte-identical to the shared `message move`
  (both call `message_move`) and works on any settled message id — the
  405 comes from Graph itself, whose `copy` is **asynchronous**: the id
  it returns points at a copy still materialising and is not yet
  operable. Not a himalaya bug; re-list the destination folder to get the
  settled id before moving it. The raw 405 is surfaced verbatim.

- `msgraph message create --folder` and `message list --folder` take a
  raw folder id or a Graph *well-known* name (`inbox`, `drafts`, …), not
  a display name — the specific commands mirror the REST path and do not
  run the shared name→id resolver (that is `msgraph.md`'s M1, a
  shared-layer convenience). This is the established raw-vs-shared split;
  the friendly path is the shared `envelope list -m <name>`.

- Graph regenerates the `Date` header on stored/sent messages (a wrong
  weekday in the submitted MIME comes back corrected), and stamps its own
  `Message-ID` on `sendMail`. Expected server behaviour.

## Verdict

The Microsoft Graph mail surface is **broad and works**: profile, mail
folders (list/get/create/rename/delete), messages
(create/list/get/update/send/copy/move/delete) and attachments
(list/get) all behave. **M2** (profile `--json`) is **fixed**. **M3**
(move a just-copied id → 405) is Graph's asynchronous-copy semantics, not
a bug. No functional blocker.
