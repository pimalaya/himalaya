---
cairn: tasks
change: envelope-search-reads-the-index
---

# Tasks

- [ ] Owner review of the proposal and its open points.
- [ ] src/pimdir/backend.rs: answer `envelope search` from `PimdirIndex::open_query` when the store has an index, the walk otherwise; the `envelope.search.sort.relevance` capability `full` there.
- [ ] Tests: the same query answered by the index and by the walk lists the same envelopes; `--sort relevance` on an indexed store; a store without an index walks.
- [ ] Spec search.md (*Per-backend translation*, the refusal of `--sort relevance`), CHANGELOG, log entry.
