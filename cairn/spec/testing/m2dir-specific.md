# m2dir *specific* API — test report

Companion to [m2dir.md](m2dir.md) (shared commands). Exercises `himalaya
m2dir …` — the raw m2dir API (store/folder lifecycle, message save,
sidecar flags).

- himalaya: `v2.0.0-alpha.1 +m2dir +rustls-ring` (working tree; `m2dir`
  is a default feature)
- backend: local `/tmp/himalaya-m2dir` ([m2dir spec][spec])
- date: 2026-07-18
- method: every `m2dir` subcommand against the throwaway store, per
  [provider-test-plan.md](provider-test-plan.md).

[spec]: https://man.sr.ht/~bitfehler/m2dir/

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `create` | `<name>`, nested `Projects/Work`, auto-inits store | ✅ (name-relative, no double-join) |
| `delete` | `<name>` | ✅ |
| `list` | base | ✅ (name + absolute path columns) |
| `messages save` | `-m <name>`, raw MIME, `-f <flag>` | ✅ |
| `flags list` | `-m <name> <id>` | ✅ |
| `flags add/set/remove` | `-f <flag>`, id before or after | ✅ (E2 fixed) |
| mailbox `rename` | — | ⚪ not offered (io-m2dir has no rename) |

## Findings

### Bugs

- **E2 — `flags add/set/remove` (and `messages save`) silently swallowed
  a trailing message id as a flag. FIXED.** `--flag` was declared
  `num_args = 1..` (greedy variadic) over a free-form `Vec<String>`, so
  `m2dir flags add -m Inbox -f seen -f flagged <id>` consumed `<id>` as a
  third flag value, leaving **zero** target ids — and, because m2dir
  flags are arbitrary strings (no enum to reject the id), it reported
  `M2dir flag(s) successfully added` while writing **nothing** (a false
  success, worse than Maildir's D5 which at least errored on its flag
  enum). `--flag` now takes exactly one value per `-f` (repeat `-f` per
  flag), mirroring the shared `flag` command, so ids can come before or
  after the flags. Verified on `add`/`set`/`remove` and `messages save`.

### Behaviour (not bugs)

- **No mailbox `rename`.** The raw `m2dir` command tree has
  `create`/`delete`/`list` but no `rename` — io-m2dir does not implement
  it yet (as noted in the CHANGELOG). Unlike Maildir, there is no rename
  to test.
- `create` / `delete` take a **name relative to the store root** and
  resolve it through the store (percent-encoding, `..`/absolute
  rejected). They do **not** pre-join the root, so there is no
  double-join bug (contrast the Maildir D3 fix); nested `Projects/Work`
  creates and deletes correctly. `create` inits the `.m2store` marker if
  missing.
- `messages save` writes to `<folder>` by **name** (`-m`, relative to the
  store root) — the same resolution the shared layer gained in E1.
  Message read/list are shared-only (no `m2dir messages read/list`).
- Flag values are arbitrary UTF-8 strings (`seen`, `$custom`), stored one
  per line in `.meta/<id>.flags`; `flags list` prints them and an empty
  set deletes the sidecar.

## Verdict

The raw m2dir surface **works**: store/folder create/delete/list, message
save and sidecar-flag list/add/set/remove all behave. **E2** (variadic
`--flag` silently ate trailing ids) is **fixed**. Mailbox `rename` is
absent by design (pending io-m2dir support). No functional blocker.
