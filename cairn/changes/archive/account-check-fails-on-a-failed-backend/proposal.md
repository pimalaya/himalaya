---
cairn: change
id: account-check-fails-on-a-failed-backend
status: landed
created: 2026-10-30
---

# account check fails when a backend fails

## Why

`account check` printed `<backend>: FAIL (...)` and exited 0 (src/account/check.rs; `a_pimdir_account_is_checked` asserted it on a root holding no store). The output contract makes the exit code the only success signal on stdout, so a script or CI step running `himalaya account check` never saw the failure. The pimdir stable-release plan of 2026-10-30 (Apps, Phase A) made calendula exit non-zero on FAIL (calendula/cairn/log/2026-10-30-account-check-fails-on-a-failed-backend.md); himalaya and cardamum follow it.

## What

- `account check` prints its report when every checked backend passed, and otherwise fails with one error naming each failed backend and why, printed (plain or JSON) as any failure is, with a non-zero exit. One value on stdout either way, as calendula does.
- The wizard, which runs its own fail-fast check on the account it builds, is unchanged.
