---
cairn: change
id: the-campaign-follow-ups
status: landed
created: 2026-11-13
---

# The campaign follow-ups

## Why

The owner settled three questions the live campaign of 2026-11-12 left open: Graph keeps every sent message in Sent Items whatever is asked, so its save options are moot; an auto-reply could not go back to having nothing set; and the composers' `--help` footer hid what `--send` and `--save` need, the same table gating the MCP tools.

## What

- A send on Graph ignores `--save`, `--no-save` and `message.send.save-copy`, reports `saved: true`, and the save of a send is checked against `submit.copy` rather than `message.add`.
- `vacation set` gains `--no-from`, `--no-to`, `--no-subject`, `--no-text` and `--no-html`, on io-jmap's clearable update fields.
- A unified leaf lists the capabilities only an argument reaches: the footer names them with their arguments, and an MCP tool leaves out an argument the account cannot serve.
