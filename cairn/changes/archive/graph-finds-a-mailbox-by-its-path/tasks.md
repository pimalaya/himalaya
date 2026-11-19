---
cairn: tasks
change: graph-finds-a-mailbox-by-its-path
---

# Tasks

- [x] src/msgraph/client.rs: `resolve_mailbox_id` and `role_mailbox_id` without the folder index; resolved ids cached, cleared by `forget_mailboxes`.
- [x] src/msgraph/backend.rs: `list_folders_under` walks toward a path when given one; `well_known_name` replaces `well_known_role`.
- [x] Tests `a_path_walks_only_the_folders_leading_to_it`, `a_path_two_folders_carry_is_refused`, `a_role_is_asked_of_its_well_known_folder`, all failing on 0cb2ff4d; `a_missing_folder_is_coded` and `a_read_of_a_message_another_folder_holds_is_not_found` scripted for the new requests.
- [x] Fold into cairn/spec/backends.md; CHANGELOG (Fixed); the log entry.
- [ ] Live cell, owed: on the Microsoft test account, `envelope list -m <nested path>`, `-m sent`, `-m deleteditems`, `-m <folder id>`, `message delete` from a folder and from Deleted Items, `message move` between two nested folders, with `RUST_LOG=debug` counting the requests.
