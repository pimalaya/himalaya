---
cairn: tasks
change: pimdir-attachment-list
---

- [x] Pin io-pimdir `14a8fca`
- [x] `PimdirClient::list_attachments`: keyset pages read one past the page, a query matched in memory
- [x] `pimdir attachment list` and `pimdir attachment search`, JSON and table, page tokens
- [x] `message parse --section`, sections numbered over the message's own multiparts
- [x] JSON Schemas registered
- [x] Tests: a store filled through io-pimdir; order, fields, ids against `envelope list`, bytes through `message read --raw` and `message parse --section`, paging, search by name and by sender, an account holding no attachments; sections against part ids
- [x] Fold into spec/backends.md and spec/commands.md, log, CHANGELOG
