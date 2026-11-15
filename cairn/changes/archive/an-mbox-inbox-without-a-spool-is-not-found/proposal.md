---
cairn: change
id: an-mbox-inbox-without-a-spool-is-not-found
status: landed
created: 2026-11-15
---

# An mbox inbox without a spool is not found

## Why

Task H23 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (mbox.md, H-5): on an mbox account without `mbox.inbox`, `envelope list` (the inbox) and `message copy -t INBOX` failed naming the key, coded `null`, where Maildir codes the same case `mailbox-not-found`, which the code table gives mbox too.

## What

`INBOX` without a spool and without a root file of that name fails coded `mailbox-not-found`, its message unchanged.
