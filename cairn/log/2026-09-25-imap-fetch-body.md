---
cairn: log
date: 2026-09-25
change: imap-fetch-body
---

# Fetch whole messages in bulk with `imap fetch --body`

A client downloading a mailbox had one way to get a message's RFC 5322 bytes: `message read --raw`, one `BODY.PEEK[]` for one UID, and a fresh process, TLS handshake and `LOGIN` per message. Yandex closes the TLS session (`close_notify`) after about sixteen logins in a row, so a first download broke part way through. `imap fetch` already covered a sequence set over one session but could not ask for the body, and `imap raw` decodes the literal lossily.

## What landed

**`imap fetch --body`** adds `BODY.PEEK[]` to the requested items. It never sets `\Seen`. The octets travel byte-exact as the standard Base64 of the JSON `body` field; the plain rendering prints the size. `--body` does not imply `--envelope`, and composes with the other item flags in the same FETCH.

**`base64` joins the `imap` feature.** It was already in the default build through `jmap`, so the dependency graph does not change.

## Capabilities moved

- commands: *IMAP fetch can return whole messages* is added

## Verification

Unit tests cover the item list for `--body` alone, with `--flags` and with no flag, and a Base64 round trip of non-UTF-8 octets. Against Yandex (`imap.yandex.ru`): for 50 messages across `INBOX` and a sent mailbox with a non-ASCII name, 9 of them 8-bit, the bytes of `imap fetch --body` equal those of `message read --raw` for every UID; flags listed before and after are identical, and the 6 unseen messages in the set stay unseen. 150 messages come down in 3.7 s in batches of 25 UIDs per call, against 0.43 s per message one `message read` at a time.
