---
cairn: spec
capability: testing
status: current
---

# Testing reports

Real-world test reports for Himalaya CLI (v2, and v3 for the reports re-run on 2026-11-18), one per backend/provider,
plus the provider-agnostic composer. Each was produced by exercising
every command variant against a live account (or a throwaway local
store), following the golden rule: **operate only inside fake mailboxes
created for the run, never on existing data**.

Method: [provider-test-plan.md](provider-test-plan.md).

The automated counterpart is the live suites, tests/google.rs, tests/microsoft.rs and tests/fastmail.rs (cairn/spec/packaging.md, "Live provider suites"): checks on the test accounts that read, save `gmail_writes` and `graph_writes`, which write only inside the labels and drafts they create and remove, run with `cargo test --test <provider> -- --ignored`.

Every report pairs a **unified** run (the cross-protocol `mailbox` /
`envelope` / `flag` / `message` / `attachment` commands) with a
**specific** run (the raw `himalaya <proto> …` surface), except where a
single combined report sufficed.

## Network backends

| Backend | Provider | Unified | Specific |
| --- | --- | --- | --- |
| IMAP + SMTP | Fastmail | [imap-smtp-fastmail.md](imap-smtp-fastmail.md) | [imap-smtp-specific-fastmail.md](imap-smtp-specific-fastmail.md) |
| IMAP + SMTP | QQ Mail (Tencent) | [imap-smtp-qq.md](imap-smtp-qq.md) | *(combined)* |
| IMAP + SMTP | 163 Mail (NetEase) | [imap-smtp-163.md](imap-smtp-163.md) | *(combined)* |
| IMAP + SMTP | 139 Mail (China Mobile) | [imap-smtp-139.md](imap-smtp-139.md) | *(combined)* |
| IMAP | Gmail (Google) | [imap-google.md](imap-google.md) | *(combined)* |
| IMAP | iCloud | [imap-icloud.md](imap-icloud.md) | *(combined; IMAP-only account)* |
| JMAP | Fastmail | [jmap-fastmail.md](jmap-fastmail.md) | [jmap-specific-fastmail.md](jmap-specific-fastmail.md) |
| Gmail REST | Google | [gmail.md](gmail.md) | [gmail-specific.md](gmail-specific.md) |
| Microsoft Graph | Outlook | [msgraph.md](msgraph.md) | [msgraph-specific.md](msgraph-specific.md) |

## Local backends

| Backend | Unified | Specific |
| --- | --- | --- |
| Maildir | [maildir.md](maildir.md) | [maildir-specific.md](maildir-specific.md) |

## Provider-agnostic

| Area | Report |
| --- | --- |
| `compose` / `reply` / `forward` (unit-tested, no provider) | [compose-reply-forward.md](compose-reply-forward.md) |

## Status

The v3 run of 2026-11-18 (master 6dd75523, release build with the default features) re-ran the Gmail, Microsoft Graph, Fastmail IMAP + SMTP, Fastmail JMAP and Maildir reports, and the three live suites passed: google 11 of 11, microsoft 11 of 11, fastmail 2 of 2. Every change of the three v3 batches the plan could reach passed against the live servers: protocol refusals printed as results with exit 3 (Gmail, Graph, JMAP sets, `smtp send`, `imap`/`smtp raw`), values refused before the session with exit 2, a send whose copy fails as one receipt with `sent: true` and exit 3, the raw single reads, the `rate-limited` code (Gmail's per-minute quota was met), relative search dates and instants, the account-level `tls` and the `account check` rows, the chunked `imap thread` over 260 messages, IMAP role resolution on reads, Gmail's or-groups and unpaged listing, and a Gmail label rename keeping its colour.

Open from that run, none fixed yet:

- A missing mailbox prints `code: null` on IMAP (against the code table: [imap-smtp-fastmail.md](imap-smtp-fastmail.md) F4), and on JMAP, Gmail and Graph (J5, G6, M4), on reads, moves, adds and the send-copy item error.
- `jmap request` refused whole (HTTP 400 `unknownCapability`) prints the failure object and exits 1 instead of the refusal object and exit 3 ([jmap-specific-fastmail.md](jmap-specific-fastmail.md) K4).
- `jmap identity create` exits 1 after the server created the identity, io-jmap 0.10.0 dropping `created` from `Identity/set` (K5, an upstream fix).

The earlier v2 runs had every bug they surfaced fixed in-tree (mailbox name to id resolution across JMAP, Gmail, Graph and Maildir, copy/move counts, CRLF and empty-body handling, the Maildir double-join and variadic-flag issues, the `has_prefix` reply/forward subject bug), with library fixes in io-imap (untagged MOVE `COPYUID`, the fallback `Date:` sort below) and io-gmail (empty 2xx bodies).


The iCloud run ([imap-icloud.md](imap-icloud.md)) surfaced two sort
issues: iCloud's server SORT is a no-op (advertised but ignores key +
REVERSE: a provider bug; workaround `imap.sort.fallback = true`), and the
io-imap client-side sort fallback ordered the `date` key by the raw
`Date:` header string lexically instead of chronologically. The latter is
**fixed** in io-imap (`cmp_fetch_items` now parses the header via
`chrono::DateTime::parse_from_rfc2822`), verified against iCloud.
Everything else on iCloud passed.

Not yet tested: Proton Bridge; SMTP on iCloud (that account is IMAP-only).

ManageSieve protocol coverage lives in io-managesieve since 2026-08-25: its
scripted server asserts the exact bytes every command sends and answers
greeting capability lines, quoted and literal script names, GETSCRIPT
literals, PUTSCRIPT warnings, activation, renaming, deletion, and logout.
This repository keeps only the tests for the `sieve` config block and its
server-URL scheme table.

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
