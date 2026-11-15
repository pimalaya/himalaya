---
cairn: delta
change: has-attachment-one-rule
---

# Delta

## ADDED Requirements

### Requirement: Backends reading the MIME structure mark attachments by the pimdir rule
IMAP, Maildir, m2dir and mbox SHALL answer `hasAttachment` by the rule of the pimdir mark (STORAGE Annex A.1) over the leaf parts of a message. A part SHALL mark the message when its `Content-Disposition` is any but `inline`. A part stating none SHALL mark it unless it is text, multipart or named by a `Content-ID`. An RFC 1847 protocol part SHALL never mark it: any child of a `multipart/encrypted`, any child but the first of a `multipart/signed`. A `Content-Type` that is no pair of RFC 2045 tokens SHALL read as `text/plain`. A `message/rfc822` part is a leaf, the parts of the attached message being its own. The `attachment` search clause SHALL read the same mark on the backends that search locally. JMAP and Graph SHALL relay what the server says; Gmail SHALL leave it `null`.

`attachment list` SHALL stay wider than this mark: it also lists signatures and images named by a `Content-ID`, and some text parts.

#### Scenario: A PDF without a disposition
- WHEN a message holds a single `application/pdf` part with no `Content-Disposition`
- THEN `envelope list --has-attachment` reports `hasAttachment: true` on IMAP, Maildir, m2dir and mbox

#### Scenario: An inline PDF
- WHEN a `multipart/mixed` message holds a text part and an `application/pdf` part with `Content-Disposition: inline`
- THEN `envelope list --has-attachment` reports `hasAttachment: false` on IMAP, Maildir, m2dir and mbox

#### Scenario: A signed message
- WHEN a `multipart/signed` message holds a text part and its signature, marked `attachment` or not
- THEN `envelope list --has-attachment` reports `hasAttachment: false` on IMAP, Maildir, m2dir and mbox
