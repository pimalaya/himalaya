---
cairn: tasks
change: envelope-in-reply-to
---

# Tasks

- [x] src/email/envelope.rs: the `in_reply_to` field, plus `parse_message_ids` reading the ids off the angle brackets that delimit them and normalising each like `message_id`.
- [x] src/imap/backend.rs: the 9th `ENVELOPE` element, at no cost beyond the FETCH the listing already issues.
- [x] src/jmap/backend.rs: `inReplyTo` added to the `Email/get` envelope properties and normalised on the way in.
- [x] src/gmail/backend.rs: one more header off the metadata payload.
- [x] src/maildir/backend.rs and src/m2dir/backend.rs: the parsed header, read through both shapes mail-parser yields for a msg-id list.
- [x] src/msgraph/backend.rs: left empty, with the reason (`internetMessageHeaders` is not in a listing selection).
- [x] src/pimdir/backend.rs: read from the `v: 1` meta and written into it by `derive_link_and_meta`, so a locally added message summarises like a synced one.
- [x] pimdir SPEC Annex A.1: `in_reply_to` as an optional array of bare msg-ids, additive at `v: 1`; log entry in the pimdir repository.
- [x] Tests: the several-parents, bracketless, empty and unterminated values, and that an id normalises to the same bytes as the parent's `message_id`.
- [x] The CHANGELOG entry.
- [x] Fold the delta into [cairn/spec/backends.md](../../spec/backends.md); write [cairn/log/2026-08-16-envelope-in-reply-to.md](../../log/2026-08-16-envelope-in-reply-to.md).
