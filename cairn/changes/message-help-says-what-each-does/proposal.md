---
cairn: change
id: message-help-says-what-each-does
status: landed
created: 2026-11-12
---

# The message help says what each subcommand does

## Why

`himalaya message --help` listed its subcommands alphabetically, the readers, the builders, the raw senders and the mailbox operations mixed together, and their help said less than they do. On 2026-11-12 the owner asked to group them and to describe them accurately: `compose`, `reply` and `forward` are builders printing, saving or sending a message, not senders; "Read a message" said nothing of what `read` prints; `parse` did not say it reads a file or its contents on stdin.

The help was also wrong in places: `--send` claimed a copy needs `--save`, though `message.send.save-copy` files one; the JSON template was said to lack "the signature that sending appends", though printing and saving append it too; `message send` named SMTP and JMAP alone, though Gmail and Microsoft Graph send through their API and pimdir queues; `reply` and `forward` pointed at `message read <id>`, whose view no composer reads, for `message read --raw <ID>`.

## What

- The subcommands list by purpose: `read` and `parse`, then `compose`, `reply` and `forward`, then `send` and `add`, then `copy`, `move` and `delete`, the group help saying so in a paragraph each.
- `compose`, `reply` and `forward` read as builders ("Build a new message from flags, then print, save or send it"), their `--save`, `--no-save` and `--send` as what they do with and without each other.
- `read` fetches a message from a mailbox and prints its headers and parts; `parse` parses a raw message from a file or stdin, without an account; `send` and `add` take a raw RFC 5322 message as it is, line endings made CRLF.

## Scope / non-goals

No behaviour changes: the names, aliases, flags and outputs stay. clap prints no heading per group of subcommands, so the order and the group help carry the grouping.
