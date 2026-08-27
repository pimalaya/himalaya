# Maildir *specific* API — test report

Companion to [maildir.md](maildir.md) (shared commands). Exercises
`himalaya maildir …` — the raw Maildir API (folder lifecycle, message
store/copy/move, filename flags).

- himalaya: `v2.0.0-alpha.1 +maildir +rustls-ring` (working tree; build
  with `--features maildir`, not a default feature)
- backend: local `/tmp/himalaya-maildir`, io-maildir default fs layout
- date: 2026-07-18
- method: every `maildir` subcommand against the throwaway tree, per
  [provider-test-plan.md](provider-test-plan.md).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `list` | base | ✅ (name + absolute path columns) |
| `create` | `<name>`, nested `Projects/Work` | ✅ (lands under root, D3 fixed) |
| `rename` | `--maildir <name> <new>` | ✅ (D3 fixed) |
| `delete` | `--maildir <name>` | ✅ (D3 fixed) |
| `messages save` | `-m <folder>`, raw MIME | ✅ (defaults subdir `new`) |
| `messages copy` | `-m <src> -t <dst> <id>` | ✅ |
| `messages move` | `-m <src> -t <dst> <id>` | ✅ |
| `flags list` | — | ✅ (code/name legend) |
| `flags add/set/remove` | `-f seen/flagged/replied …`, ids before or after | ✅ (D5 fixed) |

## Findings

### Bugs

- **D3 — `create` / `rename` / `delete` operated under `<root>/<root>`.
  FIXED.** Each command pre-joined the account root onto the folder name
  and passed the resulting **absolute** path to io-maildir, which then
  re-joined the root (relative-name contract, concatenating join). So
  `maildir create Trash` reported success but actually created
  `/tmp/himalaya-maildir/tmp/himalaya-maildir/Trash` — silently
  polluting the store's own `tmp/`. The three commands now pass the bare
  root-relative name. Verified: `create Trash` / `create Projects/Work`
  land at `<root>/Trash` and `<root>/Projects/Work`, `rename Trash Junk`
  and `delete Junk` operate on the right directory, and the store's
  `tmp/` stays empty. Same root cause as the shared-layer D1.

### Behaviour / usability

- **D4 — the specific message/flag commands default `-m` to a literal
  `Inbox` folder. DOCUMENTED.** With no `-m`, `maildir message save` /
  `flags …` resolve `Inbox` and fail `path <root>/Inbox is not a
  directory` in the fs layout, where the root *is* INBOX and no `Inbox`
  subfolder exists (the default is a Maildir++ assumption). The raw
  commands intentionally do not consult the configurable `inbox` alias.
  The default is kept, but the `-m` help now states it: pass `-m .` for
  the root maildir, or a real subfolder.
- **D5 — `flags add/set/remove --flag` swallowed a trailing message id.
  FIXED.** `--flag` was declared `num_args = 1..` (greedy variadic), so
  `flags add -m Drafts -f seen -f flagged <id>` consumed `<id>` as a flag
  value (`invalid value '<id>' for '--flag'`). It now takes exactly one
  value per `-f` (repeat `-f` per flag), mirroring the shared `flag`
  command, so ids can come before or after the flags. Flag values are
  lowercase (`passed`, `replied`, `seen`, `trashed`, `draft`,
  `flagged`); `flags list` prints the code/name legend.
- `messages save` defaults its target subdir to `new` (the shared
  `message add` uses `cur`); copied/moved messages get a fresh Maildir id
  on delivery.

## Verdict

The raw Maildir surface **works** once the double-join bug is fixed:
folder create/rename/delete, message save/copy/move and filename-flag
add/set/remove all behave. **D3** (folders created under `<root>/<root>`)
and **D5** (variadic `--flag` ate trailing ids) are **fixed**; **D4**
(default `-m Inbox`) is kept by design and now **documented** in the
`-m` help.
