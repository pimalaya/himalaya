---
cairn: log
change: maildir-custom-keywords
landed: 2026-08-16
---

# Maildir custom keywords

Maildir accounts gained `maildir.dovecot-keywords` and `maildir.keywords-header`, both off by default. The first resolves the lowercase info-section slot letters through the `dovecot-keywords` file at the root, the second reads keywords from `X-Keywords` or `X-Label`. With both off the flag set is what it was.

Custom keywords were invisible: the read path mapped the six standard letters and dropped every other character, so `envelope list "flag NonJunk"` matched nothing where the same search works on IMAP, JMAP and Graph. The shared model has carried custom keywords all along, and every other backend reads them back through `Flag::from_raw`. Maildir has no single keyword convention, and a slot letter means nothing without the sidecar defining it, so the mechanism is named rather than guessed.

The read path now calls io-maildir (`MaildirFlags::with_dovecot`, `extract_keywords_header`) instead of `parse_filename_flags` and `flag_from_char`. That drops a copy which disagreed with the library, splitting the info section at the last comma where io-maildir uses the `:2,` marker; listing covers `new/` as well as `cur/`, and a name in `new/` has no info section. The sidecar load is skipped when the option is off, so the default path costs no extra syscall, and runs once per listing rather than per entry. An unreadable sidecar warns and yields an empty table rather than failing the listing.

Deliberately not done: no CLI surface for writing a custom keyword, `FlagArg` being a closed four-variant enum, so this is read parity rather than a round trip.

Known limitation, upstream and pre-existing: on io-maildir 0.2.1 the entry locate behind `flag add` and `flag remove` discards the slot letter, so a flag operation strips a keyword it never touched. That already happens unseen; surfacing keywords makes it visible. Pending as pimalaya/io-maildir#3, picked up on 0.2.2 under the existing `0.2` requirement.

Verified: build, fmt and clippy clean; 104 tests pass, eleven new (`src/maildir/backend.rs` had no test module). Reduced feature builds checked (`imap,smtp`, `jmap`, `maildir`), the ones catching a backend crate leaking into the config schema. End to end against a throwaway Maildir with a sidecar and an `X-Keywords` message: off, both list `\Seen` alone; on, `NonJunk` and `Later`, and `envelope search flag NonJunk` goes from no matches to one. The one clippy warning (`src/wizard/search.rs:368`) is pre-existing. Not verified against a live dovecot-written mailbox.

Spec updated: backends (ADDED: Maildir surfaces custom keywords on demand; MODIFIED: Local storage backends).
