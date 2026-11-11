---
cairn: change
id: plain-text-output
status: landed
created: 2026-11-11
---

# Plain text output

## Why

The owner's direction: "pure text data, no table, no colour". The text output drew comfy-table boxes sized to the terminal, coloured by `*.list.table.*-color` keys, truncated under `-w/--max-width`, preceded by a blank line, and a few Display impls ended without a newline. A table border is noise for `grep` and `awk`, a colour escape is noise in a pipe and in a `--json` error string (ariadne drew its query errors in colour whenever stdout was a terminal), and a truncated cell loses data. pimalaya-cli now ships `columns::Columns` and `fields::Fields` for the family, and drops its `table` module.

## What

- Every list prints through `Columns`: an uppercase header, one item per line, columns two spaces apart, an empty cell `-`, no border, no colour, no truncation, no blank line before it. The free-text column moves last (an envelope list ends with SUBJECT, a mailbox list with NAME); the other columns stay. A multi-valued cell joins with `,` so a line still splits on blanks; `jmap thread get` lists one email per row; a FLAGS cell holds only the glyphs that apply, so a flagless message reads `-`.
- Every record prints through `Fields`, one aligned `Label: value` line per field: the KEY/VALUE tables (`imap status`, `imap id`, `vacation get|set`, `jmap vacation get`, `sieve capability`) and the hand-rolled blocks (`account check`, the Gmail and Microsoft Graph `get`s, `imap select`, `imap fetch`, the header block of `message read` and `message parse`). The composer template stays the RFC 5322 template an editor round-trips.
- Every text Display ends with a newline, raw bytes staying byte-exact; `jmap email parse` separates its bodies.
- The colour keys, `table.preset` and `table.arrangement` leave the configuration; crossterm leaves the dependencies; `-w/--max-width` leaves the commands that had it.
- clap loses its default features (colour, terminal-width wrapping): help prints plain and unwrapped. ariadne renders without colour always.
- `table::sanitize` becomes `sanitize::text`.

## Scope / non-goals

The `--json` contract is unchanged: same keys, same shapes, the schemas differing only in descriptions that said "table". `envelope.list.table.*-char` stays, being glyphs, not colours. ariadne keeps its box-drawing characters. A config holding a removed key is refused where its table denies unknown keys and ignored at the top and account levels, which tolerate unknown keys for himalaya-tui.
