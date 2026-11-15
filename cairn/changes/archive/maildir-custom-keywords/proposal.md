---
cairn: change
id: maildir-custom-keywords
status: landed
created: 2026-08-16
---

# Maildir custom keywords

## Why

Custom (non-IANA) keywords such as `NonJunk` are invisible on Maildir. `envelope list "flag NonJunk"` matches on IMAP, JMAP and Graph and silently matches nothing here, even when the keyword is in the mailbox.

The shared model already carries them: `Flag` keeps the raw spelling with `iana: None`, and every other backend reads them back through `Flag::from_raw`. Maildir is the only one filtering through a closed table, mapping the six standard info-section letters and dropping the lowercase slot letters that dovecot, mbsync and OfflineIMAP use.

Maildir has no single keyword convention, which is why this cannot be inferred. A keyword lives either in the mailbox's own `dovecot-keywords` file, mapping a slot letter to a name, or inline in an `X-Keywords` (OfflineIMAP, mbsync) or `X-Label` (mutt, notmuch) header. A slot letter means nothing without the sidecar defining it, and guessing a header risks inventing flags, so both have to be opt-in and named.

## What

Two account options under `maildir.keywords`, both off by default: `dovecot` resolves slot letters through the mailbox's own sidecar, `header` names the header to read. With both unset the flag set is unchanged.

Only the options live here. io-maildir's client already owned both settings and honoured them on store, so the read half belongs there too: its read paths (`read_entry`, `read_entries`, `read_entries_par`, `get`) take the Maildir the entries were listed from and hand back entries whose flags are already resolved, and himalaya reads `MaildirFullEntry::flags`. What this repo keeps is the config surface, the two assignments handing it to the client, and the `MaildirFlag` to shared `Flag` mapping. Deciding what a Maildir name means is storage semantics, which the contributing guide puts in the library that owns the format; it is also what neverest and the replica work need, and none of them would get it from a copy living here.

## Scope / non-goals

No CLI surface for writing a custom keyword. `FlagArg` is a closed four-variant `ValueEnum`, so no invocation can name one, on any backend. This is read parity; widening the flag argument is a separate cross-backend question.

`maildir.keywords.header` is read and append-on-store only, io-maildir draining keywords before the header is consulted on a flag store. Nor is reading a round trip: since no command can name a keyword, a `FlagOp::Set` store replaces the whole set and drops whatever the message carried.

## Upstream prerequisite

Both halves of this are unreleased io-maildir work, so the requirement moves from `0.2` to `0.3` when that is published, and a `[patch.crates-io]` block stands in until then.

The read API is new in 0.3. The other half is pimalaya/io-maildir#3, merged but untagged: on 0.2.1 the entry locate behind every flag store discards the slot letters before renaming, so a flag operation strips a keyword it never touched, `message read --seen` included, since it marks seen through the same path.
