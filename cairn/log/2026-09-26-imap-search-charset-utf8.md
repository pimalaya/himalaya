---
cairn: log
change: imap-search-charset-utf8
landed: 2026-09-26
---

# IMAP search sends CHARSET UTF-8 again

Reported in io-imap#3: a search with non-ASCII text failed against Gmail with `BAD Could not parse command`, because the `SEARCH` command carried no charset. It regressed #635, fixed in imap-client and lost when it became io-imap.

io-imap 0.6.1 always sends `CHARSET UTF-8`, as its SORT and THREAD already did. Himalaya only bumps the dependency to 0.6.1.

Spec unchanged.
