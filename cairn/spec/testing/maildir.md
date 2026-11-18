---
cairn: spec
capability: testing
status: current
---

# Maildir (local): unified-command test report

Unified commands on the Maildir backend. Companion: [maildir-specific.md](maildir-specific.md) (the `maildir …` raw API). Method: [provider-test-plan.md](provider-test-plan.md), against a throwaway Maildir tree.

- himalaya: release build of master c66b8629 (the three v3 batches of 2026-11-18), default features (`maildir` is one)
- backend: local filesystem, io-maildir fs layout (the root is INBOX, subfolders are nested directories with their own `cur`/`new`/`tmp`)
- date: 2026-11-18
- fixture: an empty store in the session's scratchpad, config:

  ```toml
  [accounts.maildir]
  maildir.root = "<scratchpad>/stores/maildir"
  mailbox.alias.inbox = "INBOX"
  ```

  with the six messages of the network runs (dated 3 hours to 10 days before it, one `multipart/mixed` with `note.txt`).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `account check` | text, `--json` | ✅ `tls` `null` |
| `mailbox list` / `create` | `--json` | ✅ (id = path) |
| `envelope list` | default, `.`, absolute root, `inbox` alias, `-s 2 -p 2`, `-r`, empty folder | ✅ date-descending |
| `envelope list` | missing folder | ✅ `mailbox-not-found`, exit 1 |
| `envelope search` | `today`, `yesterday`, `2 days ago`, `1 week ago`, `5 hours ago` after, before, under `or` and `not` | ✅ |
| `envelope search` | `from … and (… or attachment)`, `subject rep` (substring), flags, `body`, `attachment`, `message-id` | ✅ |
| `envelope search` | `date 1 hour ago` | ✅ refused `search-invalid` |
| `message add` | `-f` flags, `--json` | ✅ `S`/`F`/`RS` on disk |
| `message read` | pretty, multipart, `--raw`, `--raw --json`, unknown id | ✅ (`message-not-found`) |
| `flag add/set/remove` | multi-flag, multi-id, set replaces, unknown id beside a real one | ✅ item error, exit 3 |
| `message copy` / `move` | multi-id, unknown id, missing target | ✅ item error exit 3; missing target `mailbox-not-found` |
| `attachment list` / `download` | `--json` | ✅ file 0600 |
| `message compose` / `reply` | `--save B` | ✅ |
| `message send` | no sending backend | ✅ refused `capability-missing` |
| `message delete` | no trash folder | ✅ refused `capability-missing`, naming `mailbox.alias.trash` |

## Findings

### Bugs

- None.

### Behaviour (not bugs)

- Maildir is the backend where a missing mailbox is coded `mailbox-not-found` on every path the run tried; the network backends print `code: null` there (imap-smtp-fastmail.md F4, jmap-fastmail.md J5, gmail.md G6, msgraph.md M4).
- Text matching is a case-insensitive substring (`subject rep` finds both reports), and instants compare to the second under `or` and `not` as on the root, the evaluator being local.
- `message delete` needs a trash: with none marked it is refused before acting, naming the key to set.
- `message send` needs a sending block (SMTP, JMAP, Gmail, Graph or pimdir).

## Verdict

Unified commands on Maildir are **solid**: every list, search (relative dates and instants included), add, read, flag, copy, move, attachment and compose cell passes, with coded refusals. No bug found.
