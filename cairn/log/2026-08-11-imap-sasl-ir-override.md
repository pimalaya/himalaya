---
cairn: log
change: imap-sasl-ir-override
landed: 2026-08-11
---

# IMAP SASL-IR override

Himalaya accounts gained `imap.sasl-ir`, forcing the RFC 4959 initial response on or off for every SASL mechanism. Unset (the default) keeps following the advertised `SASL-IR` capability, `false` waits for the server's continuation request instead of inlining credentials with `AUTHENTICATE`, and `true` always inlines.

Fixes pimalaya/himalaya#729: Coremail (NetEase 126.com and 163.com) advertises `SASL-IR` and then answers the inline form with `BAD Request not ending with`, so those accounts could not authenticate at all. Because the capability itself is false, the client has no signal of its own to correct with, which is why this is an explicit option rather than detection. It follows `imap.sort.fallback`, the existing override of the same shape. The option sits on `imap` rather than under `imap.sasl`, both because `imap.sasl` is an externally-tagged mechanism enum with `deny_unknown_fields` (a sibling key there would be read as a seventh mechanism) and because the defect is in the server's command parser, not in one mechanism.

Deliberately not done: no automatic retry on a tagged `BAD` (it would re-send credentials after a failure and only helps servers that answer `BAD` rather than `BYE`), and nothing for SMTP (RFC 4954 section 4 puts the optional initial response in the `AUTH` grammar itself, so there is no capability to advertise or misadvertise; io-smtp always inlines and never consults EHLO for it).

Upstream io-imap: the client connect now takes an options struct carrying starttls, auto-ID and the new SASL-IR override, replacing the tail of positional arguments it had accumulated. Breaking for io-imap, which stays unpublished on 0.4 for now, so himalaya reaches it through the patch git override.

Also fixed, unrelated and pre-existing: the pimdir backend test building an envelope from meta was stale since the public id moved to seq (pimdir-public-id). It built an item without the seq field io-pimdir now carries, and expected the envelope id to be the link id rather than the seq.

Verified: build, fmt and clippy clean on both crates; io-imap's 207 tests pass and himalaya's 84 pass. Not verified against a live Coremail account, having none.

Spec updated: provider-quirks (ADDED: SASL-IR capability is overridable, Coremail advertises SASL-IR falsely).
