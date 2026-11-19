---
cairn: tasks
change: the-id-lists-scale
---

# Tasks

- [x] src/shared/flag/arg.rs: `MessageIdsArg::ids` through a `HashSet`.
- [x] src/email/outcome.rs: `Outcome::sorted_by` through a position map.
- [x] src/maildir/backend.rs: `locate_all` through `locate_many`.
- [x] No regression test (complexity alone, no timing run); `ids_are_required_and_a_repeated_one_counts_once`, `unknown_ids_are_item_errors_before_any_write_and_repeats_count_once` and the suite pass.
- [x] CHANGELOG (Fixed); the log entry.
- [ ] Left upstream: io-maildir's flag, copy, move and delete helpers locate their entry again, one folder read per id written; a write taking the located path would need an io-maildir change.
