---
cairn: log
change: source-empty-is-coded
landed: 2026-11-26
---

# An empty pimdir source is coded

io-pimdir 1.0.0 added `PimdirError::SourceEmpty`, `source-empty`, which `PimdirStore::for_source` answers for an empty source id. Himalaya relays it as every other store code it maps: `ErrorCode::SourceEmpty`, spelled `source-empty`, listed by the `himalaya-error` schema.

io-pimdir 1.0.0's other new code, `target-unsynced`, is not relayed: it is `PimdirMutateError::Unsynced`, which only the sync engine's drain meets (`PimdirMutate`, run by the store's owner), never a `PimdirError::code()` value. A move or copy Himalaya queues into a mailbox no source syncs parks, its row's `error` carrying the engine's text.

## What landed

- src/error.rs: `ErrorCode::SourceEmpty` and its `PIMDIR_CODES` entry; `an_empty_source_is_coded_source_empty`, which fails without the entry (`None`), beside `a_relayed_pimdir_code_keeps_io_pimdirs_spelling`.
- tests/json_schema.rs: `every_schema_id_names_the_v3_contract`, every schema `json-schema --dir` writes carrying `https://pimalaya.org/schemas/himalaya/v3/<name>.json`.
- CHANGELOG.md: `source-empty` among the store codes of 3.0.0.

## Capabilities moved

- [commands](../spec/commands.md): the code table gains `source-empty`.
- [backends](../spec/backends.md): the relayed pimdir codes list `source-empty`.
