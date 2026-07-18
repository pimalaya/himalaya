# Maildir (local) — shared-command test report

Shared cross-protocol commands on the Maildir backend. Companion:
[maildir-specific.md](maildir-specific.md) (the `maildir …` raw API).
Method: [provider-test-plan.md](provider-test-plan.md), against a
throwaway Maildir tree in `/tmp`.

- himalaya: `v2.0.0-alpha.1 +maildir +rustls-ring` (working tree).
  **`maildir` is not a default feature** (the default local backend is
  `m2dir`); build with `cargo build --features maildir`.
- backend: local filesystem, io-maildir default **fs layout** (root is
  itself INBOX, subfolders are real nested dirs with their own
  `cur`/`new`/`tmp`, not the dot-prefixed Maildir++ layout).
- date: 2026-07-18
- fixture: `/tmp/himalaya-maildir` with `cur`/`new`/`tmp` at the root
  and three subfolders `Archive` / `Sent` / `Drafts`, config:

  ```toml
  [accounts.maildir]
  maildir.root = "/tmp/himalaya-maildir"
  mailbox.alias.inbox = "/tmp/himalaya-maildir"
  ```

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `mailbox list` | base | ✅ (id = path, name = last segment) |
| `envelope list` | by name, by `.`, by **absolute id**, by `inbox` alias, default, `--json` | ✅ (absolute id / alias fixed, D1) |
| `envelope search` | `-- subject <term>` | ✅ (client-side filter) |
| `message add` | into root (`-m .`) and subfolders | ✅ |
| `message read` | pretty, `--json` | ✅ |
| `flag add/set/remove` | seen/flagged/answered → `S`/`F`/`R` on disk | ✅ |
| `message copy` / `move` | `--from`/`--to`, counts | ✅ (new id on delivery) |
| `attachment list` / `download` | list, `-d` | ✅ |
| `message send` | no send backend, no SMTP | ⚪ bails cleanly |

## Findings

### Bugs

- **D1 — the mailbox id from `mailbox list` (and an absolute `inbox`
  alias) were not usable as a selector. FIXED.** `mailbox list` prints
  each mailbox id as its absolute path, but feeding one back
  (`envelope list -m /tmp/himalaya-maildir/Archive`) failed `path
  /tmp/himalaya-maildir//tmp/himalaya-maildir/Archive is not a
  directory`, and `mailbox.alias.inbox = "/tmp/himalaya-maildir"`
  (the natural way to make the root the default mailbox) failed the same
  way. Root cause: io-maildir resolves every logical name **relative to
  the store root**, and himalaya's `resolve_maildir` re-joined the root
  onto an already-absolute path (io-maildir's `MaildirFsPath::join`
  concatenates rather than replacing on an absolute component, so
  `<root> + <root>/Archive`). `resolve_maildir` now reduces an absolute
  path to its root-relative name first (the empty name maps back to the
  root/INBOX). Verified: `envelope list` now works by relative name, by
  the absolute id from `mailbox list`, by the absolute root, by the
  `inbox` alias, and as the implicit default.

### Behaviour (not bugs)

- Maildir **supports the shared `envelope search`** (it lists then
  filters/sorts client-side against the already-read bytes), unlike the
  Gmail and Graph backends which bail.
- The root maildir is INBOX; `mailbox list` shows its `name` as the root
  directory's basename (e.g. `himalaya-maildir`). Select it by the
  `inbox` alias, by `.`, or by its full path — not by that basename
  (there is no `<root>/<basename>` folder). Subfolders are addressed by
  their relative name (`-m Archive`, `-m Projects/Work`).
- `message send` bails with `No send-capable backend (JMAP/Gmail/Graph)
  or SMTP is configured` — correct: Maildir cannot send, and the fixture
  has no `[smtp]` block. Add `[smtp]` to send from a Maildir account.
- `flag`/`copy`/`move` write straight to the Maildir filename info
  section (`:2,` + `S`/`R`/`F`/`D`/`T`/`P`) and re-deliver copied/moved
  messages under a fresh id, as Maildir requires.

## Verdict

Shared commands on Maildir are **solid**: list/search/add/read/flag/
copy/move/attachment all work, addressing folders by relative name, by
the absolute id from `mailbox list`, or the root by alias/`.`/path (D1
fixed). `message send` correctly needs an `[smtp]` block. No functional
blocker.
