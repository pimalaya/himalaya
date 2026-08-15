---
cairn: change
id: send-missing-date-header
status: active
created: 2026-08-15
---

# Send missing Date header

## Why

A message sent through the shared send path without a `Date:` header
is delivered as-is: nothing between the user's bytes and the wire adds
one. Receiving IMAP servers then display the message at the Unix epoch
(1970-01-01), which breaks sorting, threading and search for the
recipient.

Reproduced on v1.2.0 with `himalaya template send < message.eml`
where `message.eml` carries no `Date:`; the delivered copy arrives
stamped 1970-01-01. On master the same gap exists: `message send`
pipes raw bytes through `handler::route` into `EmailClient::send_message`
untouched, the built-in composers (`compose` / `reply` / `forward`)
never set a `Date:` either, and none of the backend adapters (SMTP,
JMAP, Gmail, Graph) inject one.

RFC 5322 §3.6 lists the origination date field as one of the two
mandatory header fields (with `From:`).

## What

`EmailClient::send_message` — the single choke point every shared send
path funnels through — prepends `Date: <now>` (RFC 5322 date-time with
local UTC offset, via `chrono::Local`) when the message carries no
`Date:` header. An existing `Date:` is never modified: a message that
already has one passes through byte-identical. The injected line
follows the message's own line-ending convention (CRLF or LF).

The `smtp send` plumbing command is deliberately out of scope: it is
an explicit-envelope raw transaction tool and stays byte-exact.

## Scope / non-goals

No `Message-ID:` injection. Also recommended by RFC 5322 §3.6, but a
separate decision with its own trade-offs (some MTAs add one
themselves); kept out to keep this change minimal.
