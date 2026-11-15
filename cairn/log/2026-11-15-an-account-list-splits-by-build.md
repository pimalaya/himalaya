---
cairn: log
change: an-account-list-splits-by-build
landed: 2026-11-15
---

# An account list splits its backends by build

Task H4 of the family's cli-alignment-2026-11-15 plan (finding F5, owner ruling D5).

## What landed

- src/account/list.rs: pimalaya-cli's `AccountListOutput` and `AccountRow`; `account_row` splits the declared blocks (`AccountConfig::configured`, now `pub(crate)`) by `Backend::COMPILED` into `backends` and `notBuilt`. The text is the kit's, which follows Himalaya's, a left-out backend marked ` (not built)`.
- src/account/check.rs: pimalaya-cli's `AccountCheckOutput<ErrorCode>` and `BackendCheck::new`, each failure handed through `error::coded` first so a typed refusal keeps its code; Himalaya's copies are gone. JSON unchanged; the text sanitizes the account name and the errors.
- src/json_schema.rs registers the kit's types under the same keys.
- tests/account.rs `account_list_splits_the_declared_backends_by_build`, JSON and text through the binary, on a full build and on `--no-default-features --features rustls-ring,maildir`; it fails on the old shape (no `notBuilt`).
- CHANGELOG (**BREAKING**, reduced builds only) and MIGRATION.md.

## Capabilities moved

- [commands](../spec/commands.md): *An account list names every block* and its scenario *A backend the build leaves out*, *account check covers every declared backend*.
