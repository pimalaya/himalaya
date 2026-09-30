---
cairn: change
id: imap-fetch-body
status: landed
created: 2026-09-25
---

# Fetch whole messages in bulk with `imap fetch --body`

## Why

A client that downloads a mailbox today has one way to get the RFC 5322 bytes of a message: `message read --raw`, one `BODY.PEEK[]` for one UID. Every call is a fresh process, a fresh TLS handshake and a fresh `LOGIN`. Some providers treat that as abuse: Yandex closes the TLS session (`close_notify`) after about sixteen logins in a row, so a first download fails part way through unless the client sleeps between messages.

`imap fetch` already takes a sequence set and returns one block per message over a single session, but cannot ask for the body. `imap raw` can, but decodes the literal as UTF-8 lossily, which corrupts 8-bit messages.

## What

A `--body` flag on `imap fetch` that adds `BODY.PEEK[]` to the requested items:

- **Peek, always.** The fetch never sets `\Seen`: a bulk download reads the mailbox, not the messages.
- **Bytes, not text.** The output keeps the octets and encodes them only when serializing: the JSON `body` field holds their standard Base64, declared in the JSON Schema with `contentEncoding: base64` and `contentMediaType: message/rfc822`. The plain rendering prints the size only.
- **Composes with the other items.** `--body --flags` returns bytes and flags from the same FETCH; `--body` alone does not add `--envelope`.

## What this is not

No partial fetch, no section selection and no `--seen` counterpart. Batching, retries and pacing stay with the caller: the command fetches exactly the sequence set it is given, over one session.
