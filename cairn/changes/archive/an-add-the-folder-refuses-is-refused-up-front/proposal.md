---
cairn: change
id: an-add-the-folder-refuses-is-refused-up-front
status: landed
created: 2026-11-15
---

# An add the folder refuses is refused up front

## Why

Task H16 of the family's cli-alignment-2026-11-15 plan, with neverest's N15 (live testing 2026-11-15, reports/himalaya/part2-pimdir.md, F4): on Graph, a message queued into a folder other than Drafts was accepted, applied by `neverest drain`, then rejected by every sync, Graph creating every MIME message as a draft. neverest declared `mail.message.add` `partial` source-wide, so the gate let the add through with a warning. neverest efc2cb3 now declares it per collection: `none` source-wide, `full` on the folder of the drafts role.

## What

Nothing to change in the code: io-pimdir 0.13's gate reads a collection's own row before its source's (`load_capabilities.sql`), and Himalaya's `message add` and `pimdir message add` stage through it on the bytes in hand. With neverest's rows, an add to any folder but Drafts fails `unsupported`, naming the mailbox and the declared detail, and leaves no row and no body. The spec states it and a test on a scratch store holding those rows pins it.

The code is `unsupported`, the one a pimdir store's gate gives (backends.md, "A pimdir store failure carries its code"), as for Fastmail's one-way writes and Gmail's `answered`; `capability-missing` stays the static table's and the server's.
