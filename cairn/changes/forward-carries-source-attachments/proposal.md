---
cairn: change
id: forward-carries-source-attachments
status: active
created: 2026-10-16
---

# Proposal: forward carries the attachments of the source

## Why

Forwarding a message with attachments silently drops them: the
composed forward quotes the text body and nothing else, so the
recipient receives a quote of a message whose files they cannot read.
[#780] agreed on the shape of the fix: keep the quoted body as it is,
re-attach the source's attachments under their original file names,
and offer `--no-attachments` for a writer who wants the quote alone.

## What changes

`message forward` appends the source's attachment parts to the
composed message, each with the file name, content type and decoded
bytes the source gave it. The quoting path is untouched: the body is
composed as before and the carried parts ride after it, so a source
richer than a single text body no longer loses its files, and a plain
text source composes exactly as it did. `--no-attachments` opts out.
A reply carries nothing.

Nameless attachments are carried too. An attached email uses its decoded
subject, sanitized for filesystem use, plus `.eml`, or `attachment.eml`
when no usable subject exists. Other nameless parts use `attachment`
plus an extension known to the existing `mime_guess` dependency, when
available. This follows the maintainer's review of PR #788.

The parts are read through the shared `message::part` walker (role,
file name, MIME type, decoded bytes), the same one `message read`,
`attachment list` and `attachment download` use, so a part names the
same part in all of them.

## Impact

- `src/shared/message/builder.rs`: the source gains `no_attachments`;
  the forward appends the source's attachment parts.
- `src/shared/message/forward.rs`: the `--no-attachments` flag.
- `src/shared/message/reply.rs`: a reply passes `no_attachments`.
- `tests/message_forward.rs`: new coverage for carrying, opting out,
  and the no-attachment source.

[#780]: https://github.com/pimalaya/himalaya/issues/780

## AI disclosure

The original implementation used Hermes and an OpenCode review. Codex
prepared this review follow-up: naming, synthetic MIME regression tests,
Cairn updates, local checks and commit amendment. Human review is pending.
