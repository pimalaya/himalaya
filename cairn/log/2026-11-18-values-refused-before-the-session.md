---
cairn: log
change: values-refused-before-the-session
landed: 2026-11-18
---

# Values refused before the session

Finding HIM-24 of the family's cli-family-analysis-2026-11-15 plan, its first and fifth points.

## What landed

- The sequence sets of `imap fetch|store|copy|move`, the envelope of `jmap submission create`, an empty `msgraph messages update`, the method and the URL of `gmail request` and `msgraph request`, and `smtp send --notify` are refused with the command line, `usage-invalid`, exit 2.
- `imap status` drops `deleted`, `deletedStorage` and `highestModSeq`.
- Tests tests/early_refusal.rs `a_value_the_command_refuses_is_refused_before_the_session`, `a_bad_sequence_set_is_refused_by_the_grammar` and `the_status_prints_the_items_it_asks_for`, failing on 72c1c0ca.
- CHANGELOG (Changed and Removed, breaking), MIGRATION (v2 to v3).

## Capabilities moved

- [commands](../spec/commands.md): *A value a protocol command refuses is refused with the command line* (added), *The Gmail and Graph escape hatches are requests*, *An IMAP command prints its completion*.
