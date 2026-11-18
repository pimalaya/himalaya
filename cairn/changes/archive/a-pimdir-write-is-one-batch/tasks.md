---
cairn: tasks
change: a-pimdir-write-is-one-batch
---

# Tasks

- [x] src/pimdir/client.rs: `enqueue_all` (the per-row `enqueue` removed).
- [x] src/pimdir/backend.rs: `delete_messages`, `refile` and `stage_flag_op` through it; `warn_partials`.
- [x] Test `a_refile_refused_on_one_id_stages_none`, failing on e440b4e4 (one row left).
- [x] Fold into cairn/spec/backends.md; CHANGELOG.
