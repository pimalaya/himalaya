---
cairn: log
change: composer-editor-template
landed: 2026-09-28
---

# Composers serve editor integrations

Needed by the himalaya-vim Vim9 rewrite, which composes through `message compose/reply/forward` flags since `message send` compiles no MML.

The composers printed wire-format bytes, RFC 2047 encoded words and a quoted-printable body included, which an editor cannot show as editable text. Under `--json`, neither saving nor sending, they now print a `MessageTemplate` decoded by mail-parser, registered as the schema of all three. Its body leaves the signature out, since sending appends the configured one: the template call skips the signature resolution entirely.

Reply and forward always quoted the source, so a body already holding an edited quote carried it twice. `PostingStyle::None` sends the written body alone, which also keeps interleaved replies possible.

## Capabilities moved

- [commands](../spec/commands.md): *Composers default the From header* gains the template and the `none` posting style.

## Envelope ids stay whole

Found driving the rewritten plugin against a Maildir: `--max-width` let comfy-table truncate the ID column, so a Maildir file name came out as `1790625978.#0M4188811…` and `message read` could not locate it. The ID column is now constrained to its content width in `Envelopes`, and the header row is capped to one line like the data rows, so a narrow width no longer stacks header names vertically.
