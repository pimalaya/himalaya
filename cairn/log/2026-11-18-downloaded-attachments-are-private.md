---
cairn: log
change: downloaded-attachments-are-private
landed: 2026-11-18
---

# Downloaded attachments are private

Finding HIM-10 of the family's cli-family-analysis-2026-11-15 plan, the attachment half.

## What landed

- src/shared/attachment/download.rs: files created 0600 by a local `private_options`; it moves to pimalaya-cli's `file::private_options` once pimalaya-cli releases it (unreleased at /home/soywod/code/pimalaya/cli, not depended on).
- Test `an_attachment_is_private_to_its_owner`, failing on 002bf012.
- CHANGELOG (Fixed).

## Capabilities moved

- [commands](../spec/commands.md): *Downloaded attachments never replace an existing entry*.
