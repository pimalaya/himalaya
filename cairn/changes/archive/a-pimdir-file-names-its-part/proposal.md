---
cairn: change
id: a-pimdir-file-names-its-part
status: landed
created: 2026-11-15
---

# A pimdir file names its part

## Why

Task H31 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (reports/himalaya/part2-pimdir.md, O1): `pimdir attachment list` numbered a file by its IMAP section (`2`, `3`), the only part number the store records, while `attachment list` and `attachment download` take the part's position in the message (`3`, `4`), the id `message read` prints. Passing the listed number to `attachment download 5868 2` wrote the text body, and nothing in either output mapped one to the other.

## What

`pimdir attachment list|search` print `part` beside `section`: the id `attachment download` takes, the position of the part standing at that section in its message's body, and print it in text where the section was. It opens the bodies of the page's messages, once per message, to number them; a file whose body is no longer local, or whose section names no leaf part (a part of an attached message), prints `part` `null`, its `section` kept for `message parse --section`. The spec's "opening no body" becomes "opening only the bodies of the page's messages". The JSON gains a key, nothing goes; the text column changes.
