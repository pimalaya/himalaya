---
cairn: change
id: an-unknown-size-is-null
status: landed
created: 2026-11-15
---

# An unknown size is null

## Why

Task H22 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (msgraph-microsoft.md, H-4): every Graph envelope printed `"size":0`, an unknown size spelled as an empty message, where commands.md "Absent is null" prints `null`.

## What

The shared envelope's `size` is optional: `null` on Graph, whose message resource carries no raw size, and on JMAP and pimdir when the server or the store gives none (they printed 0 too), the byte count elsewhere. pimdir's queued envelope follows. A JSON reader taking `size` as an integer now meets `null` on Graph.
