---
cairn: change
id: envelope-search-reads-the-index
status: active
created: 2026-11-26
---

# Envelope search reads the pimdir index

## Why

On a pimdir account `envelope search` walks the mailbox and matches each item with io-pimdir's `search::mail` matcher, over its summary or its local body (cairn/spec/search.md, *Per-backend translation*), while `pimdir search` answers the same language from the store's search index. A body term reads every local body of the mailbox: about 1 s at 11,896 messages (cairn/changes/archive/follows-storage-draft-17, D2), where the index answers in milliseconds. `--sort relevance` is refused on every backend but a pimdir index, and `envelope search` never reaches one. Carried over from envelope-search-speaks-pimdir (landed 2026-11-21), whose proposal listed it and whose log left it open.

## What

- On a pimdir account whose store has a search index, `envelope search` answers as `pimdir search kind:mail` scoped to the mailbox (`collection:` the mailbox's id), printing the `envelope search` page, and `--sort relevance` is served there.
- Without an index, or when the index refuses (`index-missing`, `index-version`), the walk answers as today.
- Open: which coverage the answer states when the index lags the store (pimdir SEARCH §8.2), and whether a text term over a mailbox whose bodies are partly local keeps the walk's reading.

Additive, after v3: a minor.
