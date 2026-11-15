---
cairn: change
id: imap-sasl-ir-override
status: landed
created: 2026-08-11
---

# IMAP SASL-IR override

## Why

Reported as pimalaya/himalaya#729: authenticating against Coremail (NetEase 126.com and 163.com) fails on v2.0.0 with `AUTHENTICATE PLAIN failed: BAD Request not ending with`, and no configuration can work around it.

The server advertises the RFC 4959 `SASL-IR` capability but rejects the inline initial response it promises to accept. io-imap decides whether to inline credentials from that capability alone, so a lying server leaves the client with no usable signal and no way in. The reporter's only escape was to abandon the mechanism entirely and fall back to `sasl.login`.

Capability detection cannot be made smarter here, because the input it reads is false. The correct value is knowable only to the person in front of the server, so it has to be expressible in config. This mirrors `imap.sort.fallback`, which already exists for the same reason: an override of a capability-derived default, unset meaning trust the capability.

## What

A new `imap.sasl-ir` account option: unset (the default) follows the advertised `SASL-IR` capability, `false` never inlines and waits for the server's continuation request (Coremail), `true` always inlines for a server supporting SASL-IR without advertising it.

The option sits directly on `imap`, not under `imap.sasl`. `imap.sasl` is an externally-tagged mechanism enum with `deny_unknown_fields`, so a sibling key inside it would be read as a seventh mechanism and rejected. Placement on `imap` is also the honest one: the defect is in the server's command parser, so it applies to every mechanism equally rather than to PLAIN in particular.

Upstream, io-imap's client connect grows an options struct carrying starttls, auto-ID and the new SASL-IR override, replacing the tail of positional arguments it had accumulated.

## Scope / non-goals

No automatic retry on a tagged `BAD`. It would re-send credentials after a failure and only ever helps a server that answers `BAD` rather than `BYE`. The explicit option is deterministic and costs no extra round trip.

Nothing for SMTP. RFC 4954 section 4 builds the optional initial response into the `AUTH` grammar itself, so there is no capability to advertise and none to lie about. io-smtp always inlines and never consults EHLO for it.

The reporter's second symptom (Coremail rejecting `SELECT` without a prior `ID`) is already covered by `imap.id.auto`, so nothing changes there.
