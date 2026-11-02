---
cairn: tasks
change: pimdir-mailbox-rename-delete
---

- [x] Pin io-pimdir 908db2b
- [x] Backend `rename_mailbox` and `delete_mailbox`, anchored on the mailbox, performer named; `PimdirQueuedIntent`
- [x] `pimdir mailbox rename` and `pimdir mailbox delete`, their JSON Schemas
- [x] `pimdir queue show` help for the two kinds and their receipts
- [x] Tests: each payload; refused undeclared, without a performer, on an empty or control-character name, on an unknown mailbox; a renaming's receipt under the new id; a deletion reading `unknown` once the mailbox left; `$Junk` gated by `mail.flags.junk`
- [x] Fold into spec/backends.md, log, CHANGELOG
