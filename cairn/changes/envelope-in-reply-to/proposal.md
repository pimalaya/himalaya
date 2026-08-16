---
cairn: change
id: envelope-in-reply-to
status: landed
created: 2026-08-16
---

# Carry In-Reply-To on the shared envelope

## Why

[Issue 734](https://github.com/pimalaya/himalaya/issues/734): the IMAP `ENVELOPE` carries `In-Reply-To` as its 9th element (RFC 3501 §7.4.2), the fetch a listing already issues returns it, and the adapter drops it because the shared `Envelope` has nowhere to put it. The only way to learn a message's parent today is `get_message` plus a parse, which is a body read per row.

The field is the parent pointer a client needs to group a conversation, and it is the cheap half of threading: every backend but one already has it in the response it makes for a listing.

## What

`Envelope.in_reply_to`, a `Vec<String>` of bare msg-ids normalised exactly like `message_id`, so a reply and its parent compare byte-for-byte whatever backend surfaced them.

**A list, not an `Option<String>`.** RFC 5322 §3.6.4 gives the field as `1*msg-id`: one id is the common case and a reply to a merged thread is not. JMAP already models it as an array (`Email/inReplyTo`, `Option<Vec<String>>` in io-jmap), so a scalar would force the one backend that hands the data over correctly to truncate it.

**Sourced where it is free.** IMAP reads the `ENVELOPE` element; JMAP adds `inReplyTo` to the `Email/get` property list; Gmail reads one more metadata header from the payload it already fetched; Maildir, m2dir and pimdir's own writer read it off the parsed headers. Graph is the one gap: `In-Reply-To` lives in `internetMessageHeaders`, which a listing selection does not return, so the field stays empty rather than costing a request per row.

**Carried in the pimdir summary too.** The pimdir backend builds envelopes from the stored `v: 1` meta with no body read, and an item at `level < Full` has no local body at all, so a field the summary does not carry is a field that backend can never answer. pimdir SPEC Annex A.1 gains `in_reply_to`, additive at `v: 1`.

## What this is not

`References:` is not added. It is the field a full threading algorithm walks, and `In-Reply-To` alone is a parent pointer that breaks wherever a client omitted the header. But it is *not* in the IMAP `ENVELOPE`, so it costs an extra `BODY.PEEK[HEADER.FIELDS (REFERENCES)]` item, and it belongs to whichever change actually builds threads rather than to this one.

The field is also not given a column in `envelope list`. A column of raw msg-ids is noise in a table; it rides the JSON output, where a client consuming it lives.
