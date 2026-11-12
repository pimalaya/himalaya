---
cairn: log
change: envelopes-hang-as-trees
landed: 2026-11-12
---

# Envelopes hang as trees

`envelope list|search` print the envelopes as a tree rooted at the mailbox: per envelope, the subject on its branch, then the date, the id with its state words and the senders (`to ` and the recipients under `--recipient`); the size no longer prints. `imap thread` prints one tree per thread from fetched envelopes, its JSON entries gaining `from`, `date` and `flags`, and `pimdir thread` hangs each message under the one it replies to. `envelope.list.datetime-relative` is removed. Capabilities moved: commands (requirement "Text output is plain text" modified), config (requirement "Envelope dates MAY render relative to today" removed).
