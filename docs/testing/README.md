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
library-level fixes were in io-imap (untagged MOVE `COPYUID`) and io-gmail
(empty 2xx body parsing), both already tracked for release; everything
else was himalaya-side. Not yet tested: iCloud and Proton Bridge.
