# m2dir (local) — shared-command test report

Shared cross-protocol commands on the m2dir backend. Companion:
[m2dir-specific.md](m2dir-specific.md) (the `m2dir …` raw API). Method:
[provider-test-plan.md](provider-test-plan.md), against a throwaway
m2store in `/tmp`.

- himalaya: `v2.0.0-alpha.1 +m2dir +rustls-ring` (working tree). `m2dir`
  is the **default** local backend (unlike `maildir`).
- backend: local filesystem, the [m2dir spec][spec]: an m2store root
  marked by `.m2store`, one directory per mailbox (each with a `.m2dir`
  marker and a `.meta/` sidecar dir), messages stored content-addressed
  as `<date>,<checksum>.<nonce>` with per-message `.meta/<id>.flags`.
- date: 2026-07-18
- fixture: `/tmp/himalaya-m2dir` with `Inbox` / `Archive` / `Sent`
  folders (bootstrapped with `m2dir create`, which inits the store),
  config:

  ```toml
  [accounts.m2dir]
  m2dir.root = "/tmp/himalaya-m2dir"
  mailbox.alias.inbox = "Inbox"
  ```

[spec]: https://man.sr.ht/~bitfehler/m2dir/

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `mailbox list` | base | ✅ (id = path, name = folder) |
| `envelope list` | by **name**, by absolute id, `inbox` alias, default, `--json` | ✅ (name resolution fixed, E1) |
| `envelope search` | `-- subject <term>` | ✅ (client-side filter) |
| `message add` | into folders by name | ✅ (content-addressed id) |
| `message read` | pretty, `--json` | ✅ |
| `flag add/set/remove` | seen/flagged/answered → `.flags` sidecar | ✅ |
| `message copy` / `move` | `--from`/`--to`, counts | ✅ (get+store; flags not copied) |
| `attachment list` / `download` | list, `-d` | ✅ |
| `message send` | no send backend, no SMTP | ⚪ bails cleanly |

## Findings

### Bugs

- **E1 — shared `-m <folder-name>` did not resolve. FIXED.**
  `envelope list -m Inbox` failed `path Inbox is not a directory`; only
  the absolute path from `mailbox list` (`-m /tmp/himalaya-m2dir/Inbox`)
  worked. io-m2dir's `open_m2dir` takes a filesystem path as-is, and the
  shared backend passed the raw `-m` value straight to it, so a folder
  name never resolved (the inverse of the Maildir backend, and
  inconsistent with the raw `m2dir` commands, which *do* take a name).
  The backend now has a `resolve_m2dir`: an absolute path opens directly
  (the `mailbox list` id), a relative name is resolved under the m2store
  root first (with the spec's percent-encoding). Verified: `envelope
  list`/`flag`/`copy`/`move` now address folders by name, by the absolute
  id, by a name- or path-valued `inbox` alias, and as the default.

### Behaviour (not bugs)

- m2dir **supports the shared `envelope search`** (lists then
  filters/sorts client-side), like Maildir and unlike Gmail/Graph.
- Messages are **content-addressed**: `message add` returns a
  `<checksum>.<nonce>` id, and `copy`/`move` are emulated as get + store
  (+ delete for move). **Flags are not propagated on copy/move** (the
  destination message starts with an empty `.flags`), matching the
  retired io-email driver.
- Flags live in the `.meta/<id>.flags` sidecar, one per line; removing
  the last flag deletes the sidecar. The shared layer writes the IANA
  spelling (`\Seen`, `\Flagged`), while the raw `m2dir flags` command
  writes the bare token (`seen`) — both are valid arbitrary-string m2dir
  flags.
- `message send` bails with `No send-capable backend (JMAP/Gmail/Graph)
  or SMTP is configured` — correct: m2dir cannot send. Add `[smtp]` to
  send from an m2dir account.

## Verdict

Shared commands on m2dir are **solid**: list/search/add/read/flag/copy/
move/attachment all work, addressing folders by name (E1 fixed), by the
absolute id, or the default via the `inbox` alias. `message send`
correctly needs an `[smtp]` block. No functional blocker.
