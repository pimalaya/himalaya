---
cairn: tasks
change: account-check-fails-on-a-failed-backend
---

# Tasks

- [x] Test: `a_pimdir_account_is_checked` expects a root holding no store to fail the command, naming `pimdir` and why, with nothing printed; fails on the old exit 0.
- [x] Return the error from `AccountCheckCommand::execute` (src/account/check.rs).
- [x] CHANGELOG Fixed entry.
- [x] Fold the delta into [commands](../../spec/commands.md); log; land.
