---
cairn: change
id: the-family-releases-adopted
status: landed
created: 2026-11-13
---

# The family releases of 2026-11-13 adopted

## Why

The io-* family published a breaking release of every library on 2026-11-13, closing the family audit of that day: io-sasl 0.2.0, io-http 0.7.0, io-oauth 0.6.0, io-imap 0.9.0, io-smtp 0.8.0, io-managesieve 0.3.0, io-jmap 0.8.0, io-gmail 0.7.0, io-msgraph 0.8.0, io-maildir 0.4.0 and io-mbox 0.2.0 among them. Himalaya asked the releases before and took six of the crates from git revisions those releases now hold, so it built on none of the fixes that came after its pins and its graph could not converge with the other applications'.

The audit also recorded two defects of Himalaya's own. The SMTP envelope was cut from a header address at its last `@` and handed to io-smtp unchecked, and the header parser can leave a control character in such an address (family audit, SMTP-S1). The configuration takes `scram-sha-256` for IMAP and SMTP while the `imap` and `smtp` features left the `scram` feature of io-imap and io-smtp off, so such an account failed as an unsupported mechanism (SASL-S1).

## What

- Cargo.toml: every family requirement raised to the release of 2026-11-13; the git pins of io-gmail, io-imap, io-jmap, io-maildir, io-msgraph and io-mbox leave `[patch.crates-io]`, those of pimalaya-cli and mime-rs staying; `imap` enables `io-imap/scram` and `smtp` enables `io-smtp/scram`.
- SMTP: an envelope address is parsed by io-smtp (`SmtpMailbox::try_from`), in the unified send and in `smtp send`, whose flags are checked when the command line is read; the envelope derived from the headers refuses an address holding a control character, for the pimdir queue too.
- IMAP: the output of a command is read through one helper warning of the lines skipped for not decoding and of the server's alerts; a read whose answer held such a line and no message fails rather than report the message missing; COPY and MOVE read the `COPYUID` pairs, bounded.
- mbox: a flag change or a removal the file refuses as stale rebuilds the index and plans again, once; a keyword and a mailbox name io-mbox refuses are refused.
- JMAP: `jmap query` reads the raw method responses back into values, its output unchanged; a send warns when the implicit `Email/set` fails.
- Gmail, Graph, ManageSieve: the new signatures and variants.
- CHANGELOG; spec (backends, commands, packaging); log.

## Scope / non-goals

No new option is exposed: neither the SCRAM iteration bounds, nor `skip_refused_recipients`, nor a JMAP mailbox moved to the top level, nor a Graph `$select`. The JMAP upload and download URL templates Himalaya fills itself stay as they are. pimalaya-stream 0.3.1, not released that day, keeps io-proxy 0.1.1.
