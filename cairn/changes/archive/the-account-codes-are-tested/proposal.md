---
cairn: change
id: the-account-codes-are-tested
status: landed
created: 2026-11-15
---

# The account codes are tested through the binary

## Why

Task H12 of the family's cli-alignment-2026-11-15 plan (finding F45). `account-missing`, `account-default-missing`, `account-default-several` and `backend-not-built` were tested on the library alone (src/config.rs, src/cli.rs), not through the binary that prints them, as Cardamum's tests/account.rs does; and the refusal of a `repl` or `mcp` line inside a session was stated in cairn/spec/repl.md with no scenario and asserted only as some error.

## What

tests/account.rs runs the binary under `--json` for each account code, and on a build without `mbox` (`cargo test --no-default-features --features rustls-ring,maildir --test account`) for `backend-not-built`, by the account and by `--backend`. tests/repl.rs asserts the nested-session refusal of `repl` and `mcp`, the session going on. Only what holds today is tested: the refusal is uncoded, and moves into pimalaya-cli later, coded `usage-invalid` (plan line L1). No behaviour changes; cairn/spec/repl.md gains the scenario.
