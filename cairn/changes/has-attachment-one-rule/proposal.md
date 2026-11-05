---
cairn: change
id: has-attachment-one-rule
status: landed
created: 2026-11-02
---

# One rule for hasAttachment across the backends that read MIME

## Why

The same message reads differently per backend (himalaya#802). IMAP counts only a `Content-Disposition: attachment`, so an order sent as one PDF part lists as having no attachment while `attachment list` shows the PDF. Maildir, m2dir and mbox used mail-parser's `attachment_count()`, which includes `inline` parts; since the move to mime-rs they read RFC 8621 `hasAttachment`, which counts signatures, encrypted parts and images named by a `Content-ID`, and misses a bare image. pimdir has since gained its own rule (io-pimdir 925b8a1).

## What

- One predicate, the rule of the pimdir mark: any disposition but `inline` counts; with none, a part counts unless text, multipart or named by a `Content-ID`; an RFC 1847 protocol part never counts.
- IMAP applies it to `BODYSTRUCTURE`; Maildir, m2dir and mbox to the mime-rs tree through their shared envelope reader, on `envelope list` and `envelope search`, and the local `attachment` search clause reads the same mark.
- The mbox envelope cache gets a version, since it would otherwise keep the old answer for every message it has seen.

## Out of scope

`attachment list` stays as it is, and wider than the mark: it also lists signatures, images named by a `Content-ID`, and text parts that name a file. JMAP, Graph and pimdir answer for themselves, and Gmail metadata cannot tell.
