---
cairn: tasks
change: message-parse
---

# Tasks

- [x] `message parse [EML|-]`: no account, no configuration; same view and text rendering as `message read`, by the same code.
- [x] `--part <PART-ID>`: raw bytes, or JSON with base64 `data`; ids shared with the view.
- [x] Bounds, blank source refused.
- [x] Tests: the view equals `message read`'s on the same bytes; `--part` bytes equal `attachment download --stdout`'s; stdin and path; no configuration on disk.
- [x] Help text, JSON schema registration, spec, log, changelog.
