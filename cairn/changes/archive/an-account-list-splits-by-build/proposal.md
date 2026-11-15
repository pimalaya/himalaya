---
cairn: change
id: an-account-list-splits-by-build
status: landed
created: 2026-11-15
---

# An account list splits its backends by build

## Why

Task H4 of the family's cli-alignment-2026-11-15 plan (finding F5, owner ruling D5). Himalaya's `account list` named every declared block under `backends`, where Cardamum and Calendula name the built ones there and the others under `notBuilt`, the shape pimalaya-cli 0.5.1 now ships as `account::AccountListOutput` and `account::AccountRow`. `account check` kept its own copy of the kit's `account::AccountCheckOutput` and `account::BackendCheck`.

## What

`account list` prints the kit's `AccountListOutput`, `backends` the blocks the build serves and `notBuilt` the ones it leaves out, its text the kit's, which follows Himalaya's (no header row, the default marked) and marks a left-out backend ` (not built)`. `account check` prints the kit's `AccountCheckOutput`, each row coded through `error::coded`, its JSON unchanged, its text now sanitizing the account name and the errors. Breaking for a reduced build only: a full build lists as before, plus `"notBuilt":[]`.
