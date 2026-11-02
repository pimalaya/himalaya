---
cairn: delta
change: one-search-language
---

The clauses, the `-h` listing, both refusal codes, the translation and the Graph flag refusal were folded into cairn/spec/search.md on 2026-11-02. What remains waits for the capability registry.

## ADDED Requirements

### Requirement: What a backend searches is declared
Each backend SHALL declare, per clause and per sort key, whether it answers it fully, in part with a detail, or not at all, and the detail of its `envelope.search` capability SHALL say how it matches text. The MCP `envelope_search` description SHALL list the clauses and sort keys the account's backend answers.
