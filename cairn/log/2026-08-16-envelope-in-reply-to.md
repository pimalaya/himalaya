---
cairn: log
change: envelope-in-reply-to
landed: 2026-08-16
---

# Carry In-Reply-To on the shared envelope

[Issue 734](https://github.com/pimalaya/himalaya/issues/734) pointed at data being thrown away: the IMAP `ENVELOPE` carries `In-Reply-To` as its 9th element (RFC 3501 §7.4.2), the FETCH a listing already issues returns it, and `envelope_from` dropped it because the shared `Envelope` had nowhere to put it. Learning a message's parent meant `get_message` and a parse, a body read per row.

## What landed

`Envelope.in_reply_to`, a `Vec<String>` of bare msg-ids, beside `message_id`.

**A list rather than an `Option<String>`,** which is where the issue's proposal was corrected. RFC 5322 §3.6.4 gives the field as `1*msg-id`: one id is the common case and a reply to a merged thread is not, and JMAP already models it as an array, so a scalar would have forced the one backend that hands the data over correctly to truncate it.

**Normalised like `message_id`.** [`parse_message_ids`](../../src/email/envelope.rs) reads the ids off the angle brackets that delimit them, falling back to whitespace for a bracketless value, and runs each through `normalize_message_id`. That is the whole point of the field: a reply's entry and its parent's `message_id` have to be the same bytes, whatever the two backends did to the header.

**Sourced where it is free**, per backend: the `ENVELOPE` element (IMAP), the `inReplyTo` property added to the `Email/get` list (JMAP), one more header off the metadata payload (Gmail), the parsed message (Maildir, m2dir), the stored summary (pimdir).

**Graph is the gap, deliberately.** `In-Reply-To` lives in `internetMessageHeaders`, which a listing selection does not return, so the field stays empty rather than costing one request per row. An absent value already means unknown, so nothing downstream has to special-case it.

## The pimdir half

The pimdir backend builds envelopes from the stored `v: 1` meta and never reads a body, and an item at `level < Full` has no local body to fall back on. A field the summary does not carry is therefore a field that backend can *never* answer, on the store where threading is most useful and a fetch least available.

So pimdir SPEC Annex A.1 gained `in_reply_to` in the same pass, as an optional array, additive at `v: 1` (an absent optional field already reads as unknown, so older rows and older readers are unaffected). `derive_link_and_meta` writes it, so a message added locally summarises exactly like one a sync stored.

## Left out

`References:`, which is the field a real threading algorithm walks and the reason `In-Reply-To` alone is a parent pointer rather than a thread. It is not in the `ENVELOPE`, so it costs an extra `BODY.PEEK[HEADER.FIELDS (REFERENCES)]` item, and it belongs to whichever change actually builds threads.

A table column. A column of raw msg-ids is noise; the field rides the JSON output, where a client consuming it lives.

## Capabilities moved

- **backends**: added the requirement that the envelope carries its threading pointers, with the per-backend sourcing rule and the Graph gap.
