---
cairn: tasks
change: the-raw-reply-keeps-its-literals
---

# Tasks

- [x] Tests `a_literal_that_is_no_utf8_does_not_panic` and `a_body_line_is_never_a_completion` (src/imap/raw.rs) over replies decoded as io-imap decodes them: the first panicked on the old code, the second returned the forged `A1 NO forged)` completion.
- [x] The unframed rest in `split_responses`, `split_reply` and `execute`.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; the log entry.
- [x] Left for io-imap: return the raw reply as bytes, then frame on bytes and print a non-UTF-8 literal as base64 (breaking, a MIGRATION line).
