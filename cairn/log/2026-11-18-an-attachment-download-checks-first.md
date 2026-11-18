---
cairn: log
change: an-attachment-download-checks-first
landed: 2026-11-18
---

# An attachment download checks first

Finding HIM-15 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/shared/attachment/download.rs: ids checked first, names fitted to 255 bytes.
- Test `an_attachment_download_checks_its_ids_first_and_cuts_long_names`, failing on 002bf012.
- CHANGELOG (Fixed).

## Capabilities moved

- [commands](../spec/commands.md): *Downloaded attachments never replace an existing entry*.
