---
cairn: log
change: schema-defs-carry-no-doc-link
landed: 2026-11-20
---

# Schema defs carry no doc link

The `himalaya-pimdir-status` schema described its three row types with Rust intra-doc links (`One mailbox's row in a [`PimdirStatusOutput`].`), read from the doc comments of src/pimdir/status.rs, which *The JSON contract is described and versioned* forbids. The 2026-11-19 check of every key (the-schemas-describe-every-key) ran on a dump before `pimdir status` landed, and no test held it. Found by the pimdir backend test run of 2026-11-20.

## What landed

- src/pimdir/status.rs: `PimdirMailboxStatus`, `PimdirParkedStatus` and `PimdirRefusedStatus` say which key of `pimdir status` holds them, in JSON terms. No printed value changes.
- src/json_schema.rs: the test `no_description_carries_a_doc_link` walks every registered schema, its `$defs` included, and refuses a description holding an intra-doc link.
- cairn/spec/commands.md: *The JSON contract is described and versioned* names the `$defs` and the test.

## Checks

The test failed before the fix naming the three `$defs` of `himalaya-pimdir-status`, and no other, under the default features and `--all-features`.
