---
cairn: log
change: has-attachment-one-rule
landed: 2026-11-05
---

# One rule for hasAttachment across the backends that read MIME

`part::marks_attachment` holds the rule of the pimdir mark (STORAGE Annex A.1): any disposition but `inline` counts; with none, a part counts unless text, multipart or named by a `Content-ID`; an RFC 1847 protocol part never counts. IMAP reads these from `BODYSTRUCTURE`. Maildir, m2dir and mbox read them off the mime-rs tree through `part::has_attachment`, called by `envelope::from_raw`, replacing RFC 8621 `hasAttachment`; the local `attachment` search clause falls back to the same function. m2dir `envelope search` now fills the mark, as the other two did.

The rule went beyond the sentence in #802, a non-text part without `inline`, because that sentence named the pimdir mark as following the same rule and io-pimdir 925b8a1 then refined it. So the mark and `attachment list` differ on signatures, on images named by a `Content-ID` and on some text parts.

The mbox cache gained a `version`: it keeps envelopes by content id, so without one every message already seen would keep its old answer. A cache without the current version is rebuilt once.

The contribution was written against mail-parser and rebased by the maintainer onto the mime-rs reader (a53eab3): the predicate is unchanged, the local walk now runs over the mime-rs tree, and the shared rows run once through `envelope::from_raw` instead of once per backend. Seven of the sixteen rows fail against the RFC 8621 rule. The IMAP tests decode `BODYSTRUCTURE` answers captured from Dovecot 2.4.5, except the ones without extension data and the single-part multipart, which it does not send.

Left as they are, each rare: a duplicate `Content-Type`, which Dovecot reads by its first; a quoted `Content-Disposition`, which Dovecot answers as an empty type; a disposition on a nested multipart, which pimdir reads and this leaf walk does not; a `Content-Type` holding one `/` but no RFC 2045 tokens, which pimdir reads as given and himalaya as `text/plain`.

Capabilities moved: **backends** (Backends reading the MIME structure mark attachments by the pimdir rule: added).
