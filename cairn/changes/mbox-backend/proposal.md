---
cairn: change
id: mbox-backend
status: landed
created: 2026-09-30
---

# mbox backend

Issue #697 asks Himalaya to read the local spool (/var/mail/$USER) so it can replace `mailx`. The same backend opens mailing-list archives, Gmail Takeout exports and Thunderbird local folders. This change adds a full read-write `mbox` backend over a new I/O-free `io-mbox` crate, mirroring the Maildir backend.

## Why a new crate

No reusable library exists. mboxshell (MIT) is a read-only std binary. Stalwart's mail-parser `mailbox::mbox` (MIT OR Apache-2.0) is a read-only `BufRead` iterator that splits on any `From ` line. meli (GPL-3.0, ideas only) reads and appends but leaves flags, deletion and mailbox management unimplemented. No Rust library rewrites flags or deletes messages, so io-mbox owns that and is tested against real archives.

## io-mbox (dependency, lands first)

no_std coroutines on io-maildir's conventions, with a std `client` pump. The pieces:

- Streaming scanner in fixed-size chunks. A message starts at a strict `From_` line: the previous line is blank, then a sender token and a leniently parsed asctime date. Real archives use two spaces before the date, `MAILER-DAEMON`, `mboxrd@z` or `-` as sender, and epoch dates.
- Every variant is read (mboxo, mboxrd, mboxcl, mboxcl2, CRLF). Writes default to mboxrd.
- Flags in `Status`, `X-Status` and `X-Keywords` (c-client). Thunderbird's `X-Mozilla-Status` is read when those are absent. The `X-IMAP` pseudo message is hidden and preserved.
- A resumable `MboxIndex` so reads never rescan an unchanged file, and MTA appends rescan only the new tail.
- Locking: dotlock, then an fcntl lock (OFD on Linux).
- Rewrites follow mutt: under lock, write the tail from the first changed message to a temp file, copy it back in place, truncate, fsync. This keeps the inode and mode and needs no write access to the spool directory.

Its corpus runs invariants on every file: Message-ID agreement with mail-parser, chunk-size independence, flag round trips keeping every byte outside the flag fields, removals, append round trips in mboxo, mboxrd and mboxcl2, and index resume. It passes on 4985 messages (33 MB) of public archives and the fixtures. A 212 MB file syncs in 0.3 s with 32 MB peak memory, and interop tests pass against formail and GNU mailutils. The files are mboxshell's and mail-parser's fixtures, pinned downloads from lists.gnu.org, lists.apache.org and lore.kernel.org, and opt-in private mboxes.

## Himalaya side

A `mbox` feature (off by default, like m2dir and pimdir) and an `[accounts.<name>.mbox]` block with `root` (directory of mbox files), `inbox` (the spool, set explicitly, `"$MAIL"` expanding), `format`, `thunderbird` and `lock.*`. `src/mbox/` mirrors `src/maildir/` and implements the full shared set. Sending goes through SMTP. The scan index and the parsed envelopes are cached under the XDG cache dir.

`himalaya mbox` becomes the raw mbox command, so the shared `mailbox` command loses its `mbox` visible alias (an alias, not the command name, so no major bump; recorded under Removed).

## Risks

A buggy rewrite loses mail on a live spool. Mitigations:

- Rewrites happen only under both locks, after a rescan under lock.
- On failure the temp file is kept and named in the error.
- Appends are truncated back to the original size on error.

On Debian, users outside group `mail` cannot dotlock in /var/mail. That fails loudly, telling to skip dotlocking (`mbox.lock.dotlock = false`), and never falls back silently.
