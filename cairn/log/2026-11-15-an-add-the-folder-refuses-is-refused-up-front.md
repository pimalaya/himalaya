---
cairn: log
change: an-add-the-folder-refuses-is-refused-up-front
landed: 2026-11-15
---

# An add the folder refuses is refused up front

Task H16 of the family's cli-alignment-2026-11-15 plan, with neverest's N15 (live testing 2026-11-15, part2-pimdir.md, F4): a Graph source declared `mail.message.add` `partial` source-wide, so an add outside Drafts was queued and rejected by every sync. neverest efc2cb3 declares it `none` source-wide and `full` on the drafts folder.

## What landed

- No code: io-pimdir 0.13's gate reads a collection's own row before its source's, and `message add` and `pimdir message add` stage through it on the bytes in hand, so such an add fails `unsupported` naming the mailbox and the detail, no row and no body left. `unsupported` is the code the store's gate gives everywhere (Fastmail's one-way writes, Gmail's `answered`), `capability-missing` staying the static table's.
- src/pimdir/backend.rs test `an_add_where_the_source_adds_nothing_is_refused_up_front`, on a scratch store holding neverest's rows.

## Capabilities moved

- [backends](../spec/backends.md): *A pimdir body is written under the staging lock* states the collection row and the refusal, with a scenario.
