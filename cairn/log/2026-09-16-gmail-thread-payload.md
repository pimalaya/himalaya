---
cairn: log
change: gmail-thread-payload
landed: 2026-09-16
---

# Gmail thread payload

Fixed #750: thread JSON now retains the optional MIME payload already fetched from Gmail. Full-format reads expose nested bodies and attachment metadata; minimal reads omit the absent payload. Summary header selection and text rendering retain their existing behavior. The generated schema includes the optional recursive payload.

This corrects the thread renderer's omission without changing io-gmail. Gmail's thread endpoint has no raw format, so preserving the parsed payload makes full thread bodies available without fetching each message separately.

The shared `--format` default moves from `full` to `metadata`, which carries everything `messages get`, `drafts get` and `threads get` print, so a plain `threads get --json` no longer emits every body in the thread.

Spec updated: commands (MODIFIED: Data commands serialize their data, carving out the `gmail threads get` payload).
