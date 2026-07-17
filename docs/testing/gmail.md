# Gmail REST on Google — shared-command test report

Shared cross-protocol commands on the Gmail REST backend. Companion:
[gmail-specific.md](gmail-specific.md) (the `gmail …` raw API). Method:
[provider-test-plan.md](provider-test-plan.md), inside a throwaway Gmail
label.

- himalaya: `v2.0.0-alpha.1 +gmail +rustls-ring` (working tree)
- account: `gmail` (Gmail REST API, OAuth via `ortie token show -a
  gmail`; needs a token scoped to `https://mail.google.com/`, not
  `carddav`)
- date: 2026-07-17
- fixtures: a fake label `Himalaya-Test` (`Label_7`) with a few inserted
  messages (one `multipart/mixed` with a small `note.txt` attachment).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `mailbox list` | base (labels) | ✅ |
| `envelope list` | by label **id**, paging, `-r`, `--json` | ✅ by id; **by name fails** (G1) |
| `envelope search` | — | ⚪ Gmail has no shared search (bails cleanly) |
| `message add` | — | ⚪ Gmail has no append (bails cleanly) |
| `message read` | pretty, `--raw`, `--json` | ✅ (label ignored) |
| `flag add/set/remove` | seen/flagged → Gmail labels | ✅ |
| `message copy` / `move` | label add / add+remove, F2 counts | ✅ |
| `attachment list` / `download` | list, `-d` | ✅ (reads inline data) |
| `message send` | real send | ✅ delivered |

## Findings

### Bugs

- **G1 — shared `-m <label-name>` now resolves to a label id. FIXED.**
  `envelope list -m Himalaya-Test` used to fail `HTTP 400: Invalid
  label`; only `-m Label_7` (the opaque id) worked. Gmail labels are
  opaque-id + display-name like JMAP mailboxes, so `GmailClient` now
  gains a cached `resolve_mailbox_id` (name → id via `labels.list`, id
  passthrough), wired into `EmailClient::resolve_mailbox_id` next to the
  JMAP arm. Verified: `envelope list`/`flag`/`copy`/`move` now address
  labels by name (and ids still work).

### Behaviour (not bugs)

- `message read` works with a label name because the gmail backend
  fetches by the globally-unique message id and ignores the `-m` label
  (like JMAP `get_message`).
- `message add` and `envelope search` bail with clear messages (`Gmail
  does not support adding messages` / `… the shared envelope search`) —
  Gmail has no append and no server-side shared-query search.
- `flag`/`copy`/`move` map onto Gmail labels (seen = absence of
  `UNREAD`, flagged = `STARRED`; copy = add label, move = add + remove).

### Observation

- `message read` renders an empty `Received:` header line — Gmail stamps
  a `Received` header on inserted/sent messages that parses to empty.

## Verdict

Shared commands on Gmail are **solid** for the supported subset (Gmail
legitimately lacks shared `add`/`search`): list/read/flag/copy/move/
attachment/send all work, now addressing labels by **name or id** (G1
fixed). No functional blocker.
