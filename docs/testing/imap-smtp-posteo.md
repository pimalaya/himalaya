# IMAP + SMTP on Posteo — test report

Shared cross-protocol commands (and IMAP-specific spot-checks) on the
IMAP/SMTP backend against **Posteo** (Dovecot). Companion to the
protocol-specific surface documented in
[imap-smtp-specific-fastmail.md](imap-smtp-specific-fastmail.md), which
is provider-agnostic and was not re-run in full here. Method:
[provider-test-plan.md](provider-test-plan.md).

- himalaya: `v2.0.0-alpha.1 +imap +smtp +rustls-ring` (working tree)
- account: `posteo` — IMAP `imaps://posteo.de`, SMTP `smtps://posteo.de`,
  SASL LOGIN/PLAIN with a `pass`-stored password
- date: 2026-07-18
- server: Dovecot; `.` hierarchy separator; advertised capabilities
  `IMAP4rev1 UIDPLUS MOVE CONDSTORE IDLE QUOTA SPECIAL-USE`

> ⚠️ **Real mailbox.** This is a live personal account, so the golden
> rule was applied strictly: every mailbox/message operation happened in
> throwaway folders `Himalaya-Test` / `Himalaya-Test2` created for the
> run. The one unavoidable real-INBOX touch — an SMTP self-test — was
> removed with an **atomic per-UID `MOVE`** into the throwaway folder
> (Posteo advertises `MOVE`+`UIDPLUS`, so no folder-wide `EXPUNGE` ran on
> INBOX). Both folders were then deleted; INBOX / Sent / Trash were
> confirmed free of any test residue.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `mailbox list` | base | ✅ |
| `envelope list` | in fake folder, `--json` | ✅ (UIDs) |
| `envelope search` | `-- subject <term>` | ✅ (server IMAP SEARCH) |
| `message add` | into fake folder | ✅ |
| `message read` | pretty | ✅ |
| `flag add/set/remove` | seen/flagged/answered | ✅ |
| `message copy` / `move` | `--from`/`--to`, counts | ✅ (COPYUID/MOVE) |
| `attachment list` / `download` | list, `-d` | ✅ |
| `message send` | real SMTP send to self | ✅ delivered |
| `imap create` / `delete` | fake folders | ✅ |
| `imap select` / `status` / `flags` | fake folder | ✅ |
| `imap fetch` | `--envelope --flags` | ✅ |
| `imap search` | `--subject` (server-side UID SEARCH) | ✅ |

## Findings

No bugs. Posteo behaves like a standard Dovecot IMAP/SMTP provider and
every exercised command works; the IMAP/SMTP backend was already
validated in depth against Fastmail, and Posteo confirms provider
portability.

### Provider notes

- **Hierarchy separator is `.`** — subfolders read as `Archives.Work`,
  `Test2.Test3`, etc. A nested throwaway folder would be
  `Himalaya-Test.Sub`.
- **`MOVE` + `UIDPLUS` are supported**, so `message move` is atomic
  per-UID and `copy`/`move` report accurate counts (the untagged-COPYUID
  path exercised for Fastmail applies here too).
- SMTP send delivered to self within seconds; `message send` does not
  save a Sent copy unless `--save <MAILBOX>` is passed, so a plain send
  only reached INBOX (kept the cleanup to a single message).
- Server-side `imap search --subject` returns matching UIDs directly,
  which made the real-INBOX cleanup surgical (no need to list the inbox).

## Verdict

Posteo (IMAP + SMTP) is **fully working** across shared and
IMAP-specific commands, with no provider-specific issues. Safe to
document as a supported provider. No functional blocker.
