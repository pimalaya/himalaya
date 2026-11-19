---
cairn: log
change: graph-finds-a-mailbox-by-its-path
landed: 2026-11-19
---

# Graph finds a mailbox by its path

Finding HIM-13 of the family's cli-family-analysis-2026-11-15 plan, its Graph half; with jmap-blobs-reuse-the-session (2026-11-18) the finding is closed.

## What landed

- src/msgraph/client.rs: `resolve_mailbox_id` asks Graph for an id-shaped value, walks a path through the folders leading to it, asks a role or a well-known name as its well-known folder, and caches what it resolved; `role_mailbox_id` asks the role's well-known folder alone; the `MailboxIndex` of every folder is gone.
- src/msgraph/backend.rs: `list_folders_under` takes the path it walks toward; `well_known_name` maps a role to its folder.
- Tests `a_path_walks_only_the_folders_leading_to_it` (two requests for `Projets/Alpha`, no `$batch`, none on the second resolve), `a_path_two_folders_carry_is_refused`, `a_role_is_asked_of_its_well_known_folder`, each failing on 0cb2ff4d; two tests rescripted for the new requests.
- CHANGELOG (Fixed).
- Not run live: no account was touched. The cell owed on the Microsoft test account is in the change's tasks.

## Capabilities moved

- [backends](../spec/backends.md): *Graph folders are named by path*, *A Graph read is scoped to the mailbox given*.
