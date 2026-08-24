---
cairn: spec
capability: testing
status: current
---

# Testing reports

Real-world test reports for Himalaya CLI v2, one per backend/provider,
plus the provider-agnostic composer. Each was produced by exercising
every command variant against a live account (or a throwaway local
store), following the golden rule: **operate only inside fake mailboxes
created for the run, never on existing data**.

Method: [provider-test-plan.md](provider-test-plan.md).

Every report pairs a **shared** run (the cross-protocol `mailbox` /
`envelope` / `flag` / `message` / `attachment` commands) with a
**specific** run (the raw `himalaya <proto> …` surface), except where a
single combined report sufficed.

## Network backends

| Backend | Provider | Shared | Specific |
| --- | --- | --- | --- |
| IMAP + SMTP | Fastmail | [imap-smtp-fastmail.md](imap-smtp-fastmail.md) | [imap-smtp-specific-fastmail.md](imap-smtp-specific-fastmail.md) |
| IMAP + SMTP | Posteo | [imap-smtp-posteo.md](imap-smtp-posteo.md) | *(combined)* |
| IMAP | Gmail (Google) | [imap-google.md](imap-google.md) | *(combined)* |
| IMAP | iCloud | [imap-icloud.md](imap-icloud.md) | *(combined; IMAP-only account)* |
| JMAP | Fastmail | [jmap-fastmail.md](jmap-fastmail.md) | [jmap-specific-fastmail.md](jmap-specific-fastmail.md) |
| Gmail REST | Google | [gmail.md](gmail.md) | [gmail-specific.md](gmail-specific.md) |
| Microsoft Graph | Outlook | [msgraph.md](msgraph.md) | [msgraph-specific.md](msgraph-specific.md) |

## Local backends

| Backend | Shared | Specific |
| --- | --- | --- |
| Maildir | [maildir.md](maildir.md) | [maildir-specific.md](maildir-specific.md) |
| m2dir | [m2dir.md](m2dir.md) | [m2dir-specific.md](m2dir-specific.md) |

## Provider-agnostic

| Area | Report |
| --- | --- |
| `compose` / `reply` / `forward` (unit-tested, no provider) | [compose-reply-forward.md](compose-reply-forward.md) |

## Status

Every bug surfaced by these runs was fixed in-tree (mailbox name→id
resolution across JMAP/Gmail/Graph/Maildir/m2dir, copy/move counts,
CRLF/empty-body handling, the Maildir/m2dir double-join and variadic-flag
issues, the `has_prefix` reply/forward subject bug, …). The only
library-level fixes were in io-imap (untagged MOVE `COPYUID`; and the
fallback `Date:`-sort fix below) and io-gmail (empty 2xx body parsing),
all tracked for release; everything else was himalaya-side.

The iCloud run ([imap-icloud.md](imap-icloud.md)) surfaced two sort
issues: iCloud's server SORT is a no-op (advertised but ignores key +
REVERSE — a provider bug; workaround `imap.sort.fallback = true`), and the
io-imap client-side sort fallback ordered the `date` key by the raw
`Date:` header string lexically instead of chronologically. The latter is
**fixed** in io-imap (`cmp_fetch_items` now parses the header via
`chrono::DateTime::parse_from_rfc2822`), verified against iCloud.
Everything else on iCloud passed.

Not yet tested: Proton Bridge; SMTP on iCloud (that account is IMAP-only).

ManageSieve has local coverage in the default test suite: a fake server
exercises greeting capability lines, quoted script names, GETSCRIPT literals,
HAVESPACE/PUTSCRIPT/CHECKSCRIPT literals, activation, deletion, and logout.
The 2026-08-24 Dovecot Pigeonhole smoke test against `franz@bett.ag` passed
STARTTLS, PLAIN authentication, `CAPABILITY`, `LISTSCRIPTS`, and
`account check --backend sieve`. The initial list was empty because the
existing `.dovecot.sieve` was a regular file outside the personal store. In
an explicitly authorized migration test, Himalaya uploaded the same rules as
`main` and activated it; ManageSieve then listed `main` as active and
`dovecot.orig` as the preserved inactive copy. `GETSCRIPT` verified both
contents. No script was deleted.

The same migration path was verified on 2026-08-24 for `mail@anycast.io` on
`mail1.anycast.io`. Its pre-existing regular `.dovecot.sieve` contained the
invoice forwarding rule. Himalaya uploaded it as `main` and activated it;
Dovecot converted `.dovecot.sieve` into its managed `sieve/main.sieve`
symlink and preserved the previous file as inactive `dovecot.orig`. Himalaya
and `doveadm` returned the same script contents. No manual symlink or direct
filesystem edit was used.
