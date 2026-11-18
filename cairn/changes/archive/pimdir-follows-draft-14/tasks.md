---
cairn: tasks
change: pimdir-follows-draft-14
---

# Tasks

- [x] Cargo.toml, Cargo.lock: pimalaya-cli 0.6.1, pimalaya-config 0.6.0, io-jmap 0.10.1, io-pimdir 0.17.0, no [patch] section.
- [x] src/shared/attachment/download.rs, src/mbox/cache.rs: `pimalaya_cli::file::private_options`, the local copy removed.
- [x] src/error.rs `pimdir_error`: `PimdirError::Version` worded by `PimdirFormatRefusal`; `reinstall` removed. Test `a_store_of_another_draft_is_refused_saying_which_is_earlier` (storage-draft-11 earlier, storage-draft-15 later), `a_cancel_leaves_a_store_of_another_draft_alone`.
- [x] src/pimdir/backend.rs: `PimdirStaged`, `PimdirQueuedIntent` and `PimdirFlagged` carry a `PimdirActionHandle`; `queue_row` takes one.
- [x] src/pimdir/client.rs: `cancel_queued` takes a handle; `uncancellable` refuses `Reset` as `store-reset`.
- [x] src/pimdir/queue/show.rs, src/pimdir/queue/cancel.rs: `ROW` parsed as `PimdirActionHandle`, `row` echoed as a string.
- [x] src/pimdir/message/add.rs, send.rs, mailbox/create.rs, rename.rs, delete.rs, flag/mod.rs, index.rs, tag.rs: `row` and `copyRow` print the handle.
- [x] Tests `a_receipt_handle_names_the_store_and_a_bare_id_still_cancels`, `a_handle_of_another_store_is_refused_as_reset`.
- [x] cairn/spec/backends.md, commands.md, search.md; CHANGELOG.md; MIGRATION.md.
