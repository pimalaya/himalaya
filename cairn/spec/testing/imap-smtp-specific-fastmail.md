---
cairn: spec
capability: testing
status: current
---

# IMAP + SMTP *specific* API on Fastmail: test report

Companion to [imap-smtp-fastmail.md](imap-smtp-fastmail.md) (the unified commands). This one exercises the raw protocol commands under `himalaya imap …` and `himalaya smtp …`.

- himalaya: release build of master 944ba0ad plus the uncommitted v3 prep, `--all-features`; `--version` prints `v3.0.0`
- account: `fastmail` (Cyrus IMAP 3.15.0-alpha0, Fastmail build of 2026-11-23)
- date: 2026-11-26
- method: every `imap`/`smtp` subcommand by hand, inside the run's mailboxes (`live-2026-11-26-himalaya-A`, `-B`, `-C`/`-C2`, and `-T` holding 260 threaded messages, 65 threads of four, appended in one `imap raw` batch with `LITERAL+`), per [provider-test-plan.md](provider-test-plan.md). All deleted at the end, with the `smtp send` copy delivered to the inbox (through Trash); Inbox, Sent, Drafts and Trash were 10, 7, 0 and 28 messages before and after.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `imap id` | `-p name:himalaya` | ✅ `serverId` |
| `imap create` / `rename` / `delete` | lifecycle; delete of a missing mailbox | ✅; `NO` printed, exit 3 |
| `imap subscribe` / `unsubscribe` | | ✅ |
| `imap list` | default (LSUB) with `-p`, `-A` | ✅ (B1) |
| `imap status` | `--json` | ✅ `messages, recent, uidNext, uidValidity, unseen` |
| `imap flag` / `select` | | ✅ `select` prints `firstUnseen`, `highestModSeq`, `permanentFlags` |
| `imap close` / `unselect` | one-shot, nothing selected | ✅ `BAD` printed, exit 3 (B2) |
| `imap select sent` | the role name, not the server's | ✅ `NO`, exit 3 |
| `imap append` | `-f '\Seen' -f kw1` | ✅ `uid 14`; `kw1` joins `PERMANENTFLAGS` |
| `imap fetch` | `--envelope --flags --internal-date --size`, `--body --structure --json`, `1:3 --seq --flags` | ✅ body Base64 |
| `imap search` | `--from --seen`, `--subject`, `--since`/`--on` (`INTERNALDATE`), `--larger`, `--header Message-ID`, `--text --seq`, `--flagged --unanswered`; `--since 18-Nov-2026` | ✅; the IMAP date form refused `usage-invalid`, exit 2 (B3) |
| `imap sort` | `-S subject -r`, `-S date`, `-S from --from alice` | ✅ |
| `imap thread` | `-A references` over 260 messages, `-A orderedsubject` | ✅ 260 entries, 65 roots, every one with its envelope (HIM-23) |
| `imap store` | `--action add -f kw2`, `remove -f kw1`, `set -f '\Seen' -f '\Flagged'`, `-f '\Deleted'` | ✅ (B4) |
| `imap copy` / `move` | UID with `-m`, unknown UID | ✅ `COPYUID` read; unknown UID `status OK`, empty `sourceUids` |
| `imap expunge` | | ✅ `expunged: []` (nothing left, B4) |
| `imap fetch` / `store` / `copy` | sequence sets `x:y`, `1,,2`, empty, `x` | ✅ refused `usage-invalid`, exit 2, before the session |
| `imap raw` | `-- 'a1 NOOP'`, `-- 'a1 BOGUS'`, a batch on stdin, 260 `APPEND`s with `LITERAL+` on stdin, `-- NOOP` (no tag) | ✅ `BAD` is the result, exit 3; untagged events then completions; the tagless line `usage-invalid`, exit 2 |
| `smtp send` | `-f -t`, to self | ✅ `{mailFrom, rcptTo}`, delivered |
| `smtp send` | a recipient the server refuses (`nobody@invalid`) | NOT RUN: denied by the auto-mode classifier (unsanctioned communication) |
| `smtp send` | `--notify never --notify success`; no `--rcpt-to` | ✅ refused `usage-invalid`, exit 2 |
| `smtp raw` | `NOOP`, `BOGUS` | ✅ `{code: 250}`; `500` is the result, exit 3 |

## Findings

### Bugs / issues

- None on this surface. F5 of the unified report (`attachment download --stdout` uncoded) is the unified layer's; F4 is fixed there.

### Provider / protocol behaviour (not bugs)

- **B1: `imap list` defaults to `LSUB`**; a created mailbox shows under `-A` until subscribed.
- **B2: `close` / `unselect` need a selection**, which a one-shot command never holds; each answers `BAD Please select a mailbox first`, printed as the result with exit 3.
- **B3: `imap search --since/--on/--before` use `INTERNALDATE`** and take `YYYY-MM-DD`: `--since 2026-11-26` lists every fixture, all appended that day whatever their `Date:`.
- **B4 (holds): Fastmail expunges `\Deleted` messages when the session ends.** `imap store 7 -m live-2026-11-26-himalaya-B -f '\Deleted'` dropped the mailbox from 6 to 5 messages with no `EXPUNGE` sent, and `imap expunge` afterwards found nothing. On Fastmail a raw `\Deleted` is final once the command returns.
- `imap copy` and `imap move` default `-m` to `INBOX`: a copy run without `-m` acts on the inbox's UIDs.

### Observations

- `imap raw` takes its command after `--` (`imap raw -- 'a1 NOOP'`) or on stdin; `imap raw 'a1 NOOP'` is refused by clap. On stdin it pipelines literals too: 260 `APPEND … {n+}` commands in one batch all came back `OK` with their `APPENDUID`.
- `imap copy` of a UID the mailbox does not hold now prints `status: OK` with `uidValidity: null` and empty UID lists, where 2026-11-18 printed `no COPYUID returned`.
- `imap flag` takes the mailbox as a positional, unlike the commands taking `-m`.

## Verdict

The raw IMAP + SMTP surface is **complete and works end-to-end** on Fastmail: server refusals are results with exit 3, values the server would refuse never open a session, `imap thread` over more than 255 messages fetches every envelope, and `imap raw` batches literals. No bug found; B4 is worth knowing before storing `\Deleted` on Fastmail. The refused-recipient `smtp send` row was not run this time.
