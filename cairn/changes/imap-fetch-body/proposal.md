---
cairn: change
id: imap-fetch-body
status: landed
created: 2026-09-25
---

# Fetch whole messages in bulk with `imap fetch --body`

## Why

A client that downloads a mailbox today has one way to get the RFC 5322 bytes of a message: `message read --raw`, which is `get_message`, one `BODY.PEEK[]` for one UID. Every call is a fresh process, a fresh TLS handshake and a fresh `LOGIN`. Reading 150 messages means 150 logins, and some providers treat that as abuse: Yandex closes the TLS session (`close_notify`) after about sixteen logins in a row, so a first download fails part way through unless the client sleeps between messages.

`imap fetch` already takes a sequence set and returns one block per message over a single session, but it can only fetch envelope, structure, flags, internal date and size. The body is the one item a downloader needs and the one it cannot ask for. `imap raw` can, but it decodes the literal as UTF-8 lossily, which corrupts 8-bit messages.

## What

A `--body` flag on `imap fetch` that adds `BODY.PEEK[]` to the requested items:

- **Peek, always.** The fetch never sets `\Seen`: bulk download is a read of the mailbox, not a read of the messages, and the flag is the user's to change.
- **Bytes, not text.** The message is carried byte-exact. In JSON it is a `body` field holding the standard Base64 of the octets, because a JSON string cannot hold arbitrary bytes. The plain-text rendering prints the size only, since dumping several raw messages to a terminal helps nobody.
- **Composes with the other items.** `--body --flags` returns bytes and flags from the same FETCH; `--body` alone does not add `--envelope`, since the envelope can be parsed from the bytes.

## What this is not

No partial fetch (`BODY.PEEK[]<0.N>`), no section selection and no `--seen` counterpart: they have no user yet. Batching, retries and pacing stay with the caller, who knows its provider; the command fetches exactly the sequence set it is given, over one session.
