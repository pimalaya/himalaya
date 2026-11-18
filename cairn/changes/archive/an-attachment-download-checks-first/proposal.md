---
cairn: change
id: an-attachment-download-checks-first
status: landed
created: 2026-11-18
---

# An attachment download checks first

## Why

Finding HIM-15 of the family's cli-family-analysis-2026-11-15 plan: `attachment download` checked its part ids after writing (`download <id> 4 99` wrote part 4, then failed, and a rerun left `ok.txt` and `ok (1).txt`), and failed whole on one name past the file system's limit (300 bytes, or a 255-byte name already taken).

## What

- Every part id checked before the first file.
- A name cut to 255 bytes at a character boundary, the ` (n)` suffix and an extension under 32 bytes kept.
