---
cairn: log
change: account-check-fails-on-a-failed-backend
landed: 2026-10-30
---

# account check fails when a backend fails

`AccountCheckCommand::execute` (src/account/check.rs) fails with ``Account `<name>` failed its check: <backend>: <reason>; ...`` when any checked backend failed, instead of printing a FAIL row and exiting 0; a passing check prints the report as before. It matches calendula's and cardamum's `account check`.

Test: `a_pimdir_account_is_checked` now expects a root holding no store to fail the command with `pimdir: No pimdir store ...`; on the old code it returned `Ok(())` (`called Result::unwrap_err() on an Ok value: ()`).

Capabilities moved: **commands** (account check covers every declared backend, modified).
