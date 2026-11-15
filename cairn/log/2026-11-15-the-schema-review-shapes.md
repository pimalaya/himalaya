---
cairn: log
change: the-schema-review-shapes
landed: 2026-11-15
---

# The schema review's shapes

Round X10 of the family's cli-alignment-2026-11-15 plan: the schema review findings A7 to A10, B4 and B8 in Himalaya.

## What landed

- A7: `Vacation` prints `start` and `end` (src/email/vacation.rs), `Start:` and `End:` in text.
- A8: `MessageTemplateOutput` carries `Address` objects, `from` an `Address` or `null` (src/shared/message/handler.rs); the text template still quotes a display name holding a special.
- A9: `Envelope.subject`, `ThreadEntry.subject` and `PimdirTagged.subject` are `Option<String>`, a missing `Subject:` (an IMAP `NIL`, a Gmail payload without the header, a pimdir summary or index title left empty) printing `null`.
- A10: `PimdirAttachmentEntry` prints `{id, filename, mime, size, part, section, message, mailbox, from, date}`.
- B4: `MessageAddOutput` prints `{id, sent, queued}`.
- B8: `Flag::spelled` builds a flag from a store's letter and its IANA keyword; `maildir flag list` prints `Flag`s, `mbox flag list` a `MboxFlagRow` flattening `Flag` beside `header`, `pimdir flag list` and `pimdir tag` `Flag`s; `Flag`'s fields carry their descriptions.

## Capabilities moved

- [commands](../spec/commands.md): *Composers default the From header*, *The auto-reply is in the unified API*, *A write prints what it acted on*, *Absent is null*, *A local store flag prints its meaning beside its spelling*.
- [backends](../spec/backends.md): *pimdir lists the account's attachments*, *pimdir stages flags by their raw spelling*.
