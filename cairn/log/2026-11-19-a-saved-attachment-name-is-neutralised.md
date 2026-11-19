---
cairn: log
change: a-saved-attachment-name-is-neutralised
landed: 2026-11-19
---

# A saved attachment name is neutralised

Line C3 of pimdir's audit-2026-11-12.

## What landed

- src/shared/attachment/download.rs: `sanitize` replaces control and bidirectional formatting characters by `_`, the list §18 names copied as `BIDI`, since io-pimdir (whose `security::path_component` is public in 0.17.0) is optional in Himalaya and the download serves every backend.
- Test `replaces_control_and_bidi_characters`, failing on 0cb2ff4d (`invoice\u{202e}fdp.exe` kept).
- CHANGELOG (Fixed).
- Left: the rule in pimalaya-cli's `sanitize` module, as the audit line suggests, for every frontend.

## Capabilities moved

- [commands](../spec/commands.md): *Downloaded attachments never replace an existing entry*.
