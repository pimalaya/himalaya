---
cairn: change
id: a-move-onto-its-own-mailbox-changes-nothing
status: landed
created: 2026-11-15
---

# A move onto its own mailbox changes nothing

## Why

Task H19 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (reports/himalaya/mbox.md, H-1): `message move -f A -t A <id>` on mbox, and the raw `mbox message move -m D2 -t D2`, rewrote the file (the message moved to its end under a new `From_` line), then exited 1 "Cannot find the message appended to <root>/A": `appended_ids` looked the copy up by the offset the append returned, which removing the original shifted. The spec asked a move onto its own mailbox to change nothing on the account-wide backends and Maildir only.

## What

A move whose source and destination are one mailbox changes nothing, on every backend, and counts the ids the mailbox holds: mbox writes nothing (the raw command printing the ids given), IMAP looks the UIDs up and sends no `MOVE` nor `COPY` (a server giving the messages new UIDs otherwise), pimdir stages nothing. JMAP, Gmail, Graph and Maildir already did so.
