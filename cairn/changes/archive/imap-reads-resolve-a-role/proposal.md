---
cairn: change
id: imap-reads-resolve-a-role
status: landed
created: 2026-11-18
---

# IMAP reads resolve a role

## Why

Finding HIM-7 of the family's cli-family-analysis-2026-11-15 plan: `envelope list -m sent` found `Sent Messages` through SPECIAL-USE, but `message read -m sent`, `attachment list|download`, `reply` and `forward` sent `SELECT sent` and failed: `EmailClient::get_message` resolves no mailbox and the IMAP adapter did not either, where the JMAP, Gmail and Graph adapters resolve inside their own `get_message`.

## What

The IMAP adapter resolves the mailbox in `get_message`, the place the other adapters chose.
