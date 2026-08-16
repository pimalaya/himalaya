---
cairn: log
change: maildir-custom-keywords
landed: 2026-08-16
---

# Maildir custom keywords

Maildir accounts gained `maildir.keywords.dovecot` and `maildir.keywords.header`, both off by default. The first resolves the lowercase info-section slot letters through each mailbox's own `dovecot-keywords` file, the second reads keywords from `X-Keywords` or `X-Label`. With both off the flag set is what it was.

Custom keywords were invisible: the read path mapped the six standard letters and dropped every other character, so `envelope list "flag NonJunk"` matched nothing where the same search works on IMAP, JMAP and Graph. The shared model has carried custom keywords all along, and every other backend reads them back through `Flag::from_raw`. Maildir has no single keyword convention, and a slot letter means nothing without the sidecar defining it, so the mechanism is named rather than guessed.

The resolution itself is io-maildir's, not ours. Its client already held `dovecot_keywords` and `keywords_header` and honoured them on store while ignoring both on read, which is why this looked like a himalaya feature at all. Its read paths now take the Maildir the entries were listed from and hand back entries carrying resolved flags (`MaildirFullEntry::flags`), the composition being an I/O-free `MaildirFlags::with_keywords` the client feeds a table it loads once per call. Here that leaves the config surface, the two assignments handing it to the client, and `flag_from_maildir`; `parse_filename_flags`, `flag_from_char` and the local sidecar loading are gone, and with them a copy of the filename format that could drift from the library's.

Deliberately not done: no CLI surface for writing a custom keyword, `FlagArg` being a closed four-variant enum, so this is read parity rather than a round trip. One consequence is documented rather than fixed: since no command can name a keyword, `flag set` replaces the whole set and drops whatever the message carried.

Unreleased dependency: both halves of the io-maildir work are untagged, so `Cargo.toml` requires `io-maildir = "0.3"`, prepared but not published, with a `[patch.crates-io]` block pointing at the local checkout until it is. The other half is pimalaya/io-maildir#3, merged but untagged: on 0.2.1 every flag store discards the slot letters, `message read --seen` included. himalaya-tui reads entries through the same API and needs the same bump.

Verified: build, fmt and clippy clean; 112 tests pass here, seven that were really testing io-maildir having moved upstream, where 80 unit tests (six new) and 15 integration tests (seven new, in tests/keyword_reads.rs) pass alongside 15 doc-tests. The one clippy warning (`src/wizard/search.rs:368`) is pre-existing. The three reduced feature builds pass, the ones catching a backend crate leaking into the config schema. Contributed as pimalaya/himalaya#735, whose end-to-end run against a throwaway Maildir had `envelope search flag NonJunk` go from no matches to one; not re-run after the refactor, and not verified against a live dovecot-written mailbox.

Spec updated: backends (ADDED: Maildir surfaces custom keywords on demand; MODIFIED: Local storage backends).
