---
cairn: tasks
change: pimdir-follows-storage-draft-13
---

# Tasks

- [x] Cargo.toml, Cargo.lock: io-pimdir 0.14.0, no [patch] section.
- [x] src/pimdir/queue/show.rs (`PimdirQueueRefusal`), src/pimdir/queue/list.rs (`PimdirQueueRefused`), src/pimdir/backend.rs (`refused_rows`); tests `an_applied_row_prints_the_refusals_on_its_message`, `a_pending_and_a_parked_row_keep_one_shape`, `a_refused_push_shows_on_its_row_and_beside_the_queue`.
- [x] src/error.rs `pimdir_error` and `reinstall`; tests `a_store_of_another_draft_is_refused_saying_to_reinstall_it` (draft-11 and draft-14), `a_cancel_leaves_a_store_of_another_draft_alone`; src/pimdir/index.rs test on search-draft-11.
- [x] Fold into cairn/spec/backends.md, commands.md and search.md; CHANGELOG; MIGRATION.md; the log entry.
