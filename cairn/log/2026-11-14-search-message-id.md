---
cairn: log
change: search-message-id
landed: 2026-11-14
---

# Search by Message-ID

`envelope search` gained `message-id <id>`, matching the whole id, angle brackets optional, against the envelope's normalised `Message-ID:`. Started as a generic `header <name> <pattern>` clause by Ravi Arnan in #811, narrowed to the Message-ID, the one header every backend but JMAP can match with the same meaning: IMAP `HEADER Message-ID` (a substring on the server, re-checked whole on the fetched envelopes on the chain of `and` from the root, the substring match standing under `or` and `not`), Gmail `rfc822msgid:`, Graph `$filter=internetMessageId eq '<…>'` for the clause alone, the shared evaluator on Maildir, mbox and pimdir; JMAP declares it `none` until io-jmap carries a `header` condition. `imap search --header NAME TEXT` keeps the contributor's generic `HEADER` key on the IMAP namespace. The generic header clause stays open in #797.

Tested: the parser (bare and bracketed ids, the whole keyword, an empty id refused), the rendering round trip, the evaluator (whole id only, case kept), the IMAP `HEADER` key and the root-chain ids, Gmail's `rfc822msgid:`, Graph's filter with an escaped quote and its refusal beside another clause, JMAP's refusal before connecting, and Maildir end to end (`not message-id` included). Clippy over the default, all-features and single-backend sets; both test suites.

Spec: search ("One query language", "A clause a backend does not answer is refused with a code", "Per-backend translation").
