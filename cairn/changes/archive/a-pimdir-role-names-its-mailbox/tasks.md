---
cairn: tasks
change: a-pimdir-role-names-its-mailbox
---

- [x] Test first, failing on the pass-through: on a store recording `inbox` and `trash`, `inbox` lists the inbox, the trash role names its mailbox, a collection name addresses nothing (the store keeps a role on one mailbox of an account, so the ambiguity refusal stays defensive)
- [x] `PimdirClient::resolve_mailbox_id` and `role_mailbox_id`, `hub_id` resolving through them
- [x] Shared `role_mailbox_id` and `resolve_mailbox_id` dispatch to pimdir; `MailboxIndex` ungated for pimdir
- [x] config.sample.toml, spec/backends.md, log, CHANGELOG
