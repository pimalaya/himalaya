---
cairn: log
change: imap-fetch-body
landed: 2026-09-30
---

# Fetch whole messages in bulk with `imap fetch --body`

A client downloading a mailbox had one way to get a message's RFC 5322 bytes: `message read --raw`, with a fresh process, TLS handshake and `LOGIN` per message. Yandex closes the TLS session after about sixteen logins in a row, so a first download broke part way through. Contributed in [#762].

## What landed

**`imap fetch --body`** adds `BODY.PEEK[]` to the requested items and never sets `\Seen`. `FetchedMessage` keeps the octets; they become standard Base64 only when serialized to JSON, and the schema declares `contentEncoding: base64` and `contentMediaType: message/rfc822` on the `body` field. The plain rendering prints the size. `--body` does not imply `--envelope` and composes with the other item flags in the same FETCH.

**`base64` joins the `imap` feature.** It was already in the default build through `jmap`.

## Capabilities moved

- commands: *IMAP fetch can return whole messages* is added

## Verification

Unit tests cover the item list for `--body` alone, with `--flags` and with no flag, and 8-bit octets kept intact through `from_items`. The contributor tested against Yandex (`imap.yandex.ru`): for 50 messages across `INBOX` and a sent mailbox with a non-ASCII name, 9 of them 8-bit, the bytes equal those of `message read --raw` for every UID, and unseen messages stay unseen. 150 messages come down in 3.7 s in batches of 25 UIDs, against 0.43 s per message one `message read` at a time.

[#762]: https://github.com/pimalaya/himalaya/pull/762
