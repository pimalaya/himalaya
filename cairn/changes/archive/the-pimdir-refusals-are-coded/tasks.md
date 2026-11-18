---
cairn: tasks
change: the-pimdir-refusals-are-coded
---

# Tasks

- [x] src/error.rs: `StoreAccountAmbiguous`, `StoreAccountUnknown`, `RowNotFound`, `ConfirmationRequired`, `PageTokenInvalid`, `page_token_invalid`.
- [x] src/pimdir/client.rs: `store-uncreated`, `resolve_account` over mail accounts, `uncancellable`, `row_not_found`.
- [x] src/pimdir/backend.rs: `hub_id` coded `mailbox-not-found`, `parse_id` `message-not-found`.
- [x] src/pimdir/queue/cancel.rs: refusal before the prompt, `confirmation-required`.
- [x] src/pimdir/search.rs, src/pimdir/attachment/mod.rs: `page-token-invalid`.
- [x] Tests tests/pimdir_refusals.rs.
- [x] Fold into cairn/spec/commands.md and backends.md, CHANGELOG, MIGRATION.
