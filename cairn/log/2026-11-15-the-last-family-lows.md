---
cairn: log
change: the-last-family-lows
landed: 2026-11-15
---

# The last family lows

Lines X9c, the queue row's `attempts` and D29 of the family's cli-alignment-2026-11-15 plan, round X13.

## What landed

- Every `mailbox-duplicate` refusal (IMAP, JMAP, Gmail, Graph, Maildir, mbox, pimdir, `maildir create`, `mbox create`) reads "Cannot create mailbox `x`: it exists already"; the pimdir and Graph details moved into `sources`.
- `pimdir queue list` prints `attempts` as an unsigned integer.
- `attachment list|download` take `--inline` alone in the help, `-i` kept as a hidden alias.

## Capabilities moved

- [commands](../spec/commands.md): *Unified mailbox writes* and *A part can be read without touching the disk* modified.
- [backends](../spec/backends.md): *The pimdir subcommand reads and retracts the queue* modified.
