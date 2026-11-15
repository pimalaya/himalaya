---
cairn: log
change: the-account-codes-are-tested
landed: 2026-11-15
---

# The account codes are tested through the binary

Task H12 of the family's cli-alignment-2026-11-15 plan (finding F45). No behaviour changed.

## What landed

- tests/account.rs: `account-missing` (an unknown name, and `-a default`), `account-default-missing`, `account-default-several` through the binary under `--json`; `backend-not-built` by the account and by `--backend` on a build without `mbox`, run with `cargo test --no-default-features --features rustls-ring,maildir --test account`.
- tests/repl.rs `a_session_refuses_a_session_inside_it`: `repl` and `mcp` lines answer `Cannot start a session inside a session`, the session going on. The refusal stays uncoded until pimalaya-cli takes it (plan line L1).

## Capabilities moved

- [repl](../spec/repl.md): the scenario *A session inside a session*.
