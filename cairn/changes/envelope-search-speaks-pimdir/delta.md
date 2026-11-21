---
cairn: delta
change: envelope-search-speaks-pimdir
---

# Delta

## MODIFIED Requirements

- search.md *One query language*: the query is pimdir SEARCH §8, parsed by io-pimdir; the order is `--sort`.
- search.md *Date values can be relative or instants*: §8.1's points, ranges, instants and amounts.
- search.md *Per-backend translation*: every translation over `PimdirQuery`; text predicates `partial` on substring servers.
- commands.md *Capability registry*: the `envelope.search.<option>` names become io-pimdir's predicate names.
- mcp.md: the `envelope_search` description lists the §8 fields the account serves.

## REMOVED Requirements

- The `order by` clause, the `to` sort key, the escape rules of unquoted patterns.

## ADDED Requirements
