---
cairn: log
change: an-empty-attachment-list-says-so
landed: 2026-11-12
---

# An empty attachment list says so

`attachment list` and `attachment download` print `No attachments` under the header when the message has no part to list, through the new `listing::Listing::empty`, where the header stood alone. The owner asked for it on 2026-11-12, the `--json` `[]` staying as it is. Capability moved: commands (requirement "Text output is plain text" modified).
