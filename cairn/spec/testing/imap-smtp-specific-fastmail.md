---
cairn: spec
capability: testing
status: current
---

# IMAP + SMTP *specific* API on Fastmail: test report

Companion to [imap-smtp-fastmail.md](imap-smtp-fastmail.md) (the unified commands). This one exercises the raw protocol commands under `himalaya imap …` and `himalaya smtp …`.

- himalaya: release build of master c66b8629, default features
- account: `fastmail` (Cyrus IMAP 3.13, Fastmail build of 2026-11-11)
- date: 2026-11-18
- method: every `imap`/`smtp` subcommand by hand, inside the run's mailboxes (`himalaya-v3-live-A`, `-B`, `-C`/`-C2`, `-T`), per [provider-test-plan.md](provider-test-plan.md). All deleted at the end.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `imap id` | `-p name:himalaya` | ✅ |
| `imap create` / `rename` / `delete` | lifecycle; delete of a missing mailbox | ✅; `NO` printed, exit 3 |
| `imap subscribe` / `unsubscribe` | | ✅ |
| `imap list` | default (LSUB) with `-p`, `-A` | ✅ (B1) |
| `imap status` | `--json` | ✅ `messages, recent, uidNext, uidValidity, unseen` only (HIM-24) |
| `imap flag` / `select` | | ✅ |
| `imap close` / `unselect` | one-shot, nothing selected | ✅ `BAD` printed, exit 3 (B2) |
| `imap select sent` | the role name, not the server's | ✅ `NO`, exit 3 |
| `imap append` | `-f '\Seen' -f kw1` | ✅ `UID 14` |
| `imap fetch` | `--envelope --flags --internal-date --size`, `--body --structure --json` | ✅ body Base64 |
| `imap search` | `--from --seen`; `--since 18-Nov-2026` | ✅; the IMAP date form refused, `YYYY-MM-DD` wanted |
| `imap sort` | `-S subject -r` | ✅ |
| `imap thread` | `-A references` over 260 messages, `-A orderedsubject` | ✅ all 260 entries carry their envelope (HIM-23) |
| `imap store` | `--action add -f kw2`, `-f '\Deleted'` | ✅ (B4) |
| `imap copy` / `move` | UID with `-m`, unknown UID | ✅ `COPYUID` read; unknown UID `no COPYUID returned` |
| `imap expunge` | | ✅ (nothing left, B4) |
| `imap fetch` / `store` / `copy` | sequence sets `x:y`, `1,,2`, empty | ✅ refused `usage-invalid`, exit 2, before the session |
| `imap raw` | `-- 'a1 NOOP'`, `-- 'a1 BOGUS'`, a batch on stdin | ✅ `BAD` is the result, exit 3 |
| `smtp send` | `-f -t`, to self | ✅ `{mailFrom, rcptTo}`, delivered |
| `smtp send` | a recipient the server refuses (`nobody@invalid`) | ✅ `{refusals: [{rcptTo, code: 504, text}]}`, exit 3 |
| `smtp send` | `--notify never --notify success` | ✅ refused `usage-invalid`, exit 2 |
| `smtp raw` | `NOOP`, `BOGUS` | ✅ `250`; `500` is the result, exit 3 |

## Findings

### Bugs / issues

- None on this surface. F4 of the unified report (missing mailbox uncoded) is the unified layer's.

### Provider / protocol behaviour (not bugs)

- **B1: `imap list` defaults to `LSUB`**; a created mailbox shows under `-A` until subscribed.
- **B2: `close` / `unselect` need a selection**, which a one-shot command never holds; each answers `BAD Please select a mailbox first`, now printed as the result with exit 3.
- **B3: `imap search --since/--on/--before` use `INTERNALDATE`** and take `YYYY-MM-DD`.
- **B4: Fastmail expunges `\Deleted` messages when the session ends.** `imap store 12 -m A --action add -f '\Deleted'` dropped the mailbox from 10 to 9 messages with no `EXPUNGE` sent (the trace shows `SELECT`, `STORE`, then the connection closing), and `imap expunge` afterwards found nothing. A server setting, not himalaya: on Fastmail a raw `\Deleted` is final once the command returns.
- `imap copy` and `imap move` default `-m` to `INBOX`: a copy run without `-m` acts on the inbox's UIDs.

### Observations

- `imap raw` takes its command after `--` (`imap raw -- 'a1 NOOP'`) or on stdin; the provider test plan's `raw '<TAG> <COMMAND>'` form is refused by clap.
- `imap copy` and `move` print `no COPYUID returned` for a UID the mailbox does not hold, which tells a no-op apart from a copy.

## Verdict

The raw IMAP + SMTP surface is **complete and works end-to-end** on Fastmail: the v3 contract holds (server refusals are results with exit 3, values the server would refuse never open a session, `smtp send` reports its refusals), and `imap thread` over more than 255 messages fetches every envelope. No bug found; B4 is worth knowing before storing `\Deleted` on Fastmail.
