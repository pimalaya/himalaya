# Gmail *specific* API on Google — test report

Companion to [gmail.md](gmail.md) (shared commands). Exercises
`himalaya gmail …` — the raw Gmail REST v1 surface.

- himalaya: `v2.0.0-alpha.1 +gmail +rustls-ring` (working tree)
- account: `gmail` (Gmail REST, OAuth via ortie)
- date: 2026-07-17
- method: every `gmail` subcommand, inside a fake label, per
  [provider-test-plan.md](provider-test-plan.md). Account-wide setting
  **writes** were deliberately not exercised (get/list only).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `profile get` | text + `--json` | ✅ (`--json` structured, G3 fixed) |
| `labels list/get/create/update/delete` | full lifecycle | ✅ |
| `messages list` | `--label`, `-q` | ✅ |
| `messages get` | `--format metadata/raw/full`, `--header` | ✅ |
| `messages insert` / `import` | `--label` | ✅ |
| `messages modify` / `batch-modify` | `--add/remove-label` | ✅ |
| `messages trash` / `untrash` | — | ✅ |
| `messages send` | raw message | ✅ delivered |
| `messages delete` / `batch-delete` | permanent | ✅ (used for cleanup) |
| `attachments get` | `<msg> <att-id> -o` | ⚠️ needs raw token, inline caveat (G2) |
| `drafts create/list/get/update/delete` | full lifecycle | ✅ |
| `threads list/get/modify` | `--add/remove-label` | ✅ |
| `history list` | `--start-history-id` | ✅ |
| `settings imap/pop/language/vacation/auto-forwarding get` | — | ✅ |
| `settings send-as list` | — | ✅ |
| `settings filters list` | — | ✅ (G4 fixed) |
| `settings forwarding-addresses list` | — | ✅ (G4 fixed) |
| `settings delegates list` | — | ⚪ 403 (Workspace-only) |
| `settings *` set/create/update/delete | language/imap/vacation/filters | ⚠️ 403, needs settings scope (G5) |

## Findings

### Bugs

- **G4 — empty settings lists failed to parse. FIXED (io-gmail).**
  `settings filters list` and `settings forwarding-addresses list` errored
  `Gmail response parsing failed: invalid type: null, expected struct
  GmailFiltersListResponse` when the account had none. Root cause was in
  io-gmail's `GmailSend`: an empty 2xx body was normalised to `null`,
  which fails every struct response (the list structs already carry
  `#[serde(default)]` but can't accept a top-level `null`). It now
  normalises to `{}` — `GmailNoResponse` (deletes) still ignores it, and
  list structs fall back to their defaults. Verified: both lists now
  render empty (`--json` → `{"filter":[]}`). himalaya bumped to io-gmail
  0.2 + path-patch until an io-gmail release ships this.

- **G5 — settings *writes* need the settings OAuth scope.** Every write
  (`language set`, `imap set`, `pop set`, `vacation set`,
  `auto-forwarding set`, `filters create/delete`, …) returns `HTTP 403:
  Request had insufficient authentication scopes` with a token scoped to
  `https://mail.google.com/`. Gmail settings mutations require
  `https://www.googleapis.com/auth/gmail.settings.basic` (and
  `.settings.sharing` for delegates / external forwarding), which
  `mail.google.com` does not include. Not a himalaya bug — the raw 403
  is surfaced verbatim — but the scope must be added and re-authorised to
  use the write commands. Settings **reads** work with `mail.google.com`.

### Behaviour / usability

- **G2 — `gmail attachments get` needs the raw Gmail `attachmentId`, and
  small attachments have none.** Passing the shared `attachment list`
  index (`1`) → `HTTP 400: Invalid attachment token`. The real
  `attachmentId` only comes from `messages get --format full`. Worse,
  Gmail **inlines** small attachments in `body.data` with no
  `attachmentId` at all (the test `note.txt` had none → empty id → `404
  …/attachments/`). So `gmail attachments get` only applies to larger,
  separately-stored attachments; the shared `attachment download` reads
  both inline and stored data and is the reliable path.
- **G3 — `gmail profile get --json` is now structured. FIXED.** It used
  to emit `{"message":"Email: …\n…"}` because the command formatted a
  plain `String` into a `Message` (which serializes as `{"message":
  …}`). A dedicated `GmailProfileOutput { email, messages-total,
  threads-total, history-id }` (Display + Serialize) replaces it, so
  `--json` now emits
  `{"email":"…","messages-total":85,"threads-total":67,"history-id":"…"}`
  and the text output is unchanged.
- `settings delegates list` → `HTTP 403: Access restricted to service
  accounts … domain-wide authority` — delegates need Google Workspace
  domain-wide delegation, unavailable to a normal account. Not a bug;
  the raw 403 is surfaced verbatim.

## Verdict

The Gmail REST surface is **broad and works**: profile, labels, messages
(incl. insert/import/modify/trash/send/batch/delete), drafts, threads,
history, and settings reads all behave. **G3** (profile `--json`) and
**G4** (empty `filters`/`forwarding-addresses` lists) are **fixed**.
**G5** (settings writes → 403) is an OAuth-scope requirement, not a bug —
add `gmail.settings.basic`/`.sharing` and re-authorise to exercise the
write commands. **G2** (attachment id) is a raw-API ergonomics note (see
below); delegates 403 is a Workspace limitation. No functional blocker.
