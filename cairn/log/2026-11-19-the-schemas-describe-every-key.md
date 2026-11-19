---
cairn: log
change: the-schemas-describe-every-key
landed: 2026-11-19
---

# The schemas describe every key

Findings B15, B16, C1, C4 and the closed-set part of B3 of the schema review of 2026-11-14, in the v3 pre-release pass.

## What landed

- src/json_schema.rs: five enums naming their alternatives in place of the generic `OneOf`.
- Descriptions on every key the dump showed bare, the printed ones in JSON terms.
- The read view's addresses on the shared `Address`.
- `sieve raw` `status` and `pimdir performer` `capability` typed as `enum` in their schemas; `role` and the queue `kind` kept strings, open by spec.
- No printed value changes; checked on a fresh dump (219 schemas): no undescribed key or alternative, no `None`, no intra-doc link.
- CHANGELOG (Changed).

## Capabilities moved

- [commands](../spec/commands.md): the `json-schema` paragraph.
