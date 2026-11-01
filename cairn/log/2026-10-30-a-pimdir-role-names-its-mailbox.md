---
cairn: log
change: a-pimdir-role-names-its-mailbox
landed: 2026-10-30
---

# A role name addresses its pimdir mailbox

From the pimdir stable-release review of 2026-10-30: `mailbox list` showed the roles the sync engine recorded, but no command resolved them, so `envelope list` without `-m` failed on `inbox` and `message delete` could not find the trash. The 2026-10-05 pimdir-mailbox-roles log and CHANGELOG claimed otherwise. `PimdirClient::resolve_mailbox_id` now takes a collection id of the account first, then a role name, and `role_mailbox_id` answers from the same roles; `hub_id`, which every pimdir mailbox goes through, resolves with them, so the `pimdir` subcommands take role names too, and the shared client dispatches both calls to pimdir. A collection's name addresses nothing: two sources may hold mailboxes of one name. The store keeps a role on one mailbox of an account (io-pimdir `set_collection_role`), so the refusal of a role several mailboxes carry is defensive only.

Test: `a_role_name_addresses_its_mailbox`, failing on the old code with "Mailbox `inbox` not found".

Capabilities moved: **backends** (A pimdir mailbox is its collection id, Mailbox role).
