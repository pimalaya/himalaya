---
cairn: tasks
change: a-saved-attachment-name-is-neutralised
---

# Tasks

- [x] src/shared/attachment/download.rs: `sanitize` replaces control and bidirectional formatting characters.
- [x] Test `replaces_control_and_bidi_characters`, failing on 0cb2ff4d.
- [x] Fold into cairn/spec/commands.md, *Downloaded attachments never replace an existing entry*; CHANGELOG (Fixed); the log entry.
- [x] Later, in pimalaya-cli: a shared path-component sanitizer the frontends call, Himalaya's local rule then dropped. (2026-11-19: pimalaya-cli 0.7 `sanitize::path_component`, the-releases-of-2026-11-19)
