---
cairn: log
change: imap-search-charset-when-needed
landed: 2026-10-01
---

# IMAP search sends CHARSET UTF-8 only when needed

Supersedes 2026-09-26-imap-search-charset-utf8. Always sending `CHARSET UTF-8` broke every search on Outlook, which answers `NO The specified charset is not supported` (#769), the shared search reaching it through the SORT fallback since Outlook has no SORT.

io-imap 0.7.1 encodes the `SEARCH` command and adds `CHARSET UTF-8` only when the bytes are not pure ASCII: ASCII criteria use the US-ASCII default RFC 3501 section 6.4.4 requires every server to support, non-ASCII ones keep the charset Gmail needs. SORT and THREAD keep their mandatory UTF-8 charset (RFC 5256). Himalaya bumps the dependency to 0.7.1.

A non-ASCII search on Outlook IMAP still fails, the server rejecting the UTF-8 charset; the Microsoft Graph backend searches it.

Spec unchanged.
