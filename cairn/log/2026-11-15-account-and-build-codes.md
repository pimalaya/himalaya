---
cairn: log
change: account-and-build-codes
landed: 2026-11-15
---

# Account and build codes

Owner rulings of 2026-11-15, items 13, 16 and 24 of the merged list.

## What landed

- pimalaya-config d5a33cb reads `default` as a name like any other: `-a default` names an account called `default`, its refusal adding that leaving `-a` out takes the account marked default. The test of several defaults now expects `Missing` for it.
- `account-missing`, `account-default-missing`, `account-default-several`: `error::coded` reads pimalaya-config's `AccountError` off the chain through pimalaya-cli's `with_code` (51ce8b1), applied to the result `main` reports, the MCP server's start failure and each tool result. `resolve_account` still returns the `AccountError`, which himalaya-tui and the tests read.
- `backend-not-built`: `AccountConfig::no_backend_error` names the backends the account configures and this build leaves out, when none of its blocks serves the command or `--backend` names one, with the `--features` to rebuild with. Every block already loaded in every build. The message of a `--backend` the command does not serve no longer says "or build".
- `a_backend_left_out_of_the_build_names_its_feature`, run in a build without `mbox`.
- Lock: the re-resolve downgraded windows-sys 0.61.2 to 0.52.0 under errno, rustix, tempfile and winapi-util; those lines were put back.

## Capabilities moved

- [config](../spec/config.md): `-a default` and the account codes.
- [commands](../spec/commands.md): a backend left out of the build.
