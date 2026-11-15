---
cairn: log
change: a-pimdir-file-names-its-part
landed: 2026-11-15
---

# A pimdir file names its part

Task H31 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (part2-pimdir.md, O1): `pimdir attachment list` numbered a file by its IMAP section, `attachment download` by the part's position, so the listed number downloaded another part.

## What landed

- src/pimdir/attachment/mod.rs `part_ids`, src/pimdir/attachment/list.rs: each file carries `part`, the id `attachment download` takes, read from its message's body once per message of the page, `null` when the body is not local or no leaf part stands at its section; the text prints it where the section was. src/shared/message/part.rs: `Leaf::section` readable.
- tests/pimdir_attachments.rs: `a_file_s_part_is_the_one_attachment_download_takes` downloads every listed file by its `part` and checks it against `attachment list`; `a_file_whose_body_is_gone_lists_no_part`.

## Capabilities moved

- [backends](../spec/backends.md): *pimdir lists the account's attachments* prints `part` and opens the page's bodies, with a scenario.
- [commands](../spec/commands.md): the text columns of `pimdir attachment list|search`.
