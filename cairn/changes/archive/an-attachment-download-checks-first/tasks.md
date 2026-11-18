---
cairn: tasks
change: an-attachment-download-checks-first
---

# Tasks

- [x] src/shared/attachment/download.rs: the wanted parts first; `fit_name`, `NAME_MAX`, `EXT_MAX`.
- [x] Test tests/local_stores.rs `an_attachment_download_checks_its_ids_first_and_cuts_long_names`, failing on e440b4e4 (one file written before the refusal).
- [x] Fold into cairn/spec/commands.md; CHANGELOG.
