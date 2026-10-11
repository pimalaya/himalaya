---
cairn: change
id: reply-flags-answered
status: landed
created: 2026-10-09
---

# A sent reply or forward flags its source

## Why

`message reply <id> --send` leaves the source message as it was: `\Answered` is not added, so `envelope list` and every other client keep showing a message that was replied to as one that was not (#778). The caller can add the flag with `flag add`, but only knows to do it once it has noticed, and every front-end would have to repeat it.

RFC 3501 section 2.3.2 defines `\Answered` as "message has been answered", and setting it is what a mail client does on reply. RFC 5788 registers `$Forwarded` for the same purpose on a forward, and `message forward --send` leaves the source untouched too.

## What

- `message reply --send` adds the `answered` flag to the source once the send has succeeded, through the shared `store_flags`, so each backend maps it as `flag add --flag answered` does.
- A reply only printed or saved (`--save` without `--send`) flags nothing: nothing was answered yet.
- A flag failing after the send fails the command with an error stating that the message was sent, as a failing copy does.
- `message forward --send` adds the `forwarded` flag to the source the same way. `$Forwarded` is a keyword rather than a system flag, and a server may refuse keywords it does not list in `PERMANENTFLAGS`, so this one is best-effort: a refusal is logged as a warning and the forward, already sent, still succeeds.
