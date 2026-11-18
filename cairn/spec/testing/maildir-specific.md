---
cairn: spec
capability: testing
status: current
---

# Maildir *specific* API: test report

Companion to [maildir.md](maildir.md) (unified commands). Exercises `himalaya maildir …`: the raw Maildir API (folder lifecycle, message store/copy/move, filename flags).

- himalaya: release build of master c66b8629, default features
- backend: the throwaway store of [maildir.md](maildir.md), io-maildir fs layout
- date: 2026-11-18
- method: every `maildir` subcommand against the throwaway tree, per [provider-test-plan.md](provider-test-plan.md).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `list` | | ✅ (name and absolute path) |
| `create` | `Trash`, nested `Projects/Work` | ✅ under the root |
| `rename` | `--maildir Trash Junk` | ✅ |
| `delete` | `--maildir Junk`, `--json`; missing folder | ✅ `{id}`; missing folder `mailbox-not-found`, exit 1 |
| `rename` / `delete` | the folder as a positional | ✅ refused by clap, exit 2 (`--maildir` is the flag) |
| `message add` | `-m B`, `--json`; no `-m` | ✅ `{id, path}` into `new/`; no `-m` lands in the root |
| `message copy` / `move` | `-m B -t Archive`, `-t Projects/Work` | ✅ fresh id on delivery |
| `flag list` | | ✅ code and name legend |
| `flag add/set/remove` | `-f seen -f flagged`, `-f replied`, id before `-f` | ✅ `{ids}`; the message moves from `new/` to `cur/` |

## Findings

### Bugs

- None.

### Behaviour / usability

- **D4 (kept): no `-m` reads the default `Inbox`**, which resolves to the root maildir when no top-level `Inbox` folder exists, so `maildir message add -- beta.eml` landed in the root's `new/`.
- `rename` and `delete` name their folder with `--maildir`, not a positional; the plan's `rename <name> <new>` form is refused by clap.
- `message add` stores into `new/` (the unified `message add` uses `cur/`); the first flag change moves the file to `cur/`.

## Verdict

The raw Maildir surface **works**: folder create/rename/delete, message add/copy/move and filename-flag add/set/remove all behave, the store's `tmp/` stays empty. No bug found.
