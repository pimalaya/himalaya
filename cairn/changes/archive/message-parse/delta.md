---
cairn: change
change: message-parse
---

# Delta

## ADDED Requirements

### Requirement: A message can be read without an account
`message parse [EML]` SHALL read a raw message from a path, or from stdin with `-` or when omitted, and SHALL print what `message read` prints for the same bytes. It SHALL resolve no account and read no configuration.

### Requirement: A parsed message hands one part over
`message parse --part <PART-ID>` SHALL write that part's bytes, transfer encoding undone, to stdout, or under `--json` `{id, mime, filename, size, data}` with `data` in base64; the ids SHALL be those of the view.
