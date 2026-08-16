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

Maildir has no single keyword convention, which is why this cannot be inferred. A keyword lives either in the `dovecot-keywords` sidecar at the root, mapping a slot letter to a name, or inline in an `X-Keywords` (OfflineIMAP, mbsync) or `X-Label` (mutt, notmuch) header. A slot letter means nothing without the sidecar defining it, and guessing a header risks inventing flags, so both have to be opt-in and named.

## What

Two account options on `maildir`, both off by default: `maildir.dovecot-keywords` resolves slot letters through the sidecar, `maildir.keywords-header` names the header to read. With both unset the flag set is unchanged.

The read path stops parsing filenames by hand and calls io-maildir instead. That is the layering the contributing guide asks for, deciding what a Maildir name means being storage semantics. It also drops a copy that disagreed with the library: it split the info section at the last comma, where io-maildir uses the `:2,` marker. Listing covers `new/` as well as `cur/`, and a name in `new/` has no info section at all.

Everything needed is published in io-maildir 0.2.1, so no dependency moves.

## Scope / non-goals

No CLI surface for writing a custom keyword. `FlagArg` is a closed four-variant `ValueEnum`, so no invocation can name one, on any backend. This is read parity; widening the flag argument is a separate cross-backend question.

`maildir.keywords-header` is read and append-on-store only, io-maildir draining keywords before the header is consulted on a flag store.

On io-maildir 0.2.1 the entry locate behind `flag add` and `flag remove` discards the slot letter before renaming, so a flag operation strips a keyword it never touched. The defect is upstream and pending as pimalaya/io-maildir#3; the `0.2` requirement picks the fix up on 0.2.2. Documented with that caveat rather than held back, since the read path is useful alone and the stripping already happens today, unseen.
