---
cairn: log
change: mbox-backend
landed: 2026-09-30
---

# mbox backend

Himalaya gained an mbox backend over the new io-mbox library, behind the off-by-default `mbox` feature (#697). The backends capability gained the mbox backend, its content-id addressing, flag mapping (a seen message marked old too, for GNU mail), locked in-place writes and the per-file cache of the scan index and parsed envelopes. The config capability gained the `mbox` block. The commands, packaging and search capabilities name mbox beside Maildir and m2dir.

`himalaya mbox` is now the raw mbox command, so the shared `mailbox` command lost its `mbox` visible alias. Being an alias rather than the command name, its removal is recorded under Removed without a major bump.

Wiring it up found an io-mbox bug: a CRLF message appended by `message add` got a CRLF `From_` line, and GNU `mail -H` then rewrote the file with that line quoted, merging two messages. io-mbox now stores appended messages with LF line endings. Himalaya depends on io-mbox 0.1.0 from crates.io, which carries the fix.

The wizard does not detect mbox files yet.
