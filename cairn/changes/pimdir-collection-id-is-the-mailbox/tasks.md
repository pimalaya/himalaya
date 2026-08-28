---
cairn: tasks
change: pimdir-collection-id-is-the-mailbox
---

- [x] Remove `pimdir.namespace` from `PimdirConfig` and `config.sample.toml`
- [x] Remove `resolve_namespace` and the client's `namespace` field
- [x] Remove `mailbox_name`; `Mailbox.name` carries the collection row's name
- [x] Reduce `hub_id` to an existence check refusing an unknown id by name
- [x] Verify against the real store: a known id resolves, an unknown one is refused naming what the account holds
- [x] Fold the spec, log the change, update the CHANGELOG
