---
cairn: log
change: the-schema-leftovers
landed: 2026-11-18
---

# The schema leftovers

Findings C3, C6 (its `warnings` half) and C12 of the schema review of 2026-11-14, the three still breaking a consumer after the-schema-review-shapes.

## What landed

- `pimdir mailbox list` coverage `closedAt`; `pimdir search --threads` and `pimdir thread` `firstDate` and `lastDate`.
- `sieve check|put` `warnings` as a list.
- `jmap email parse` `bodies: [{blobId, body}]` beside `errors`.
- Tests `coverage_and_round_render_as_camel_case_json` (updated), `the_warnings_are_a_list`, `each_body_names_its_blob`.
- CHANGELOG (Changed, breaking), MIGRATION (v2 to v3, development build).

## Capabilities moved

- [commands](../spec/commands.md): *ManageSieve commands*, *A command acting on several items lists what failed*.
- [backends](../spec/backends.md): *pimdir lists each mailbox with its coverage*.
- [search](../spec/search.md): *pimdir searches through the store's index*.
