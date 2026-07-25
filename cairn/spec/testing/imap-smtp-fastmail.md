# IMAP + SMTP on Fastmail — shared-command test report

- himalaya: `v2.0.0-alpha.1 +imap +smtp +rustls-ring` (rev `1dddbe8`,
  working tree with the shared-JMAP / CRLF / date fixes applied)
- account: `fastmail` (`imap.server = imaps://imap.fastmail.com`,
  `smtp.server = smtps://smtp.fastmail.com`, SASL PLAIN)
- date: 2026-07-17
- method: every shared command × every flag variant, run by hand inside
  two throwaway mailboxes (`Himalaya-Test-A` / `-B`), per
  [provider-test-plan.md](provider-test-plan.md). Fixtures: 6 messages
  (Alpha…Epsilon + Eta) with distinct sender/subject/date/flags and one
  `multipart/mixed` with a `note.txt` attachment.

## Results

| Command | Variants tested | Result |
| --- | --- | --- |
| `mailbox list` | base, `--counts`, `--max-width`, `--json`, combos | ✅ all pass |
| `envelope list` | base, `-p/-s` paging, `-r`, `--has-attachment`, `-w`, `--json`, empty mbox | ✅ pass; default `-m` → error (see F3) |
| `envelope search` | from/to/subject/body/flag/date/after, and/or/not/grouping, `order by … asc/desc`, `--json` | ✅ DSL correct; matching + date semantics are server-defined (P1, P2) |
| `message add` | inline-escape, inline-LF, file, stdin, `-f` multi, `--json` | ✅ pass; empty input (F1), missing `-m` clean error |
| `message read` | pretty, multipart, `--raw`, `--json`, bad id | ✅ pass; PEEK (no `\Seen`); `--json` is low-level (O1) |
| `flag add/set/remove` | multi-flag, multi-id, set-replaces, remove, errors | ✅ all pass |
| `message copy` | single, multi, errors, non-existent id, default source | ✅ copies; F2 + F3 |
| `message move` | single, source-emptied, errors | ✅ pass; F2 |
| `attachment list` | multipart, `-i` inline, no-attachment | ✅ all pass |
| `attachment download` | `-d`, `--json`, collision de-dupe | ✅ all pass |
| `message compose` | from/to/cc/bcc/subject/body/body-file/attach/signature[-file]/save/send | ✅ all pass |
| `message send` | raw RFC 5322 via stdin, `--save` | ✅ pass |
| `message reply` | `Re:`, threading, quote, `-P top/bottom`, `-Q` | ✅ all pass |
| `message forward` | `Fwd:`, quoted original, `--to`, `--save` | ✅ pass |

SMTP delivery confirmed end-to-end (`compose --send` and `message send`
both landed in the inbox; delivered copies moved to Trash on cleanup).

## Findings

### Bugs / issues

- **F1 — empty message is not validated client-side. FIXED.**
  `message add -m "$A" -- ''` used to reach the server and fail with
  `IMAP APPEND failed: NO Zero-length message literal`. `MessageArg`
  now resolves the message first and rejects an empty result uniformly
  (positional `''`, empty file, empty stdin) with `Error: Message is
  empty`; a TTY with no input gives `No message provided: …`.

- **F2 — no-op copy/move reported success. FIXED.** IMAP `UID COPY`/
  `UID MOVE` of a UID that does not exist is a server no-op that returns
  `OK`, so himalaya used to print `Message(s) successfully copied/moved`
  although nothing moved. `copy_messages`/`move_messages` now return the
  **actual affected count** and the command prints it: `N message(s)
  successfully copied`, or `No message copied: no id matched in the
  source mailbox` when zero. Sources of truth per backend: IMAP reads
  the UIDPLUS `COPYUID` source set (absent ⇒ 0 on a UIDPLUS server, else
  the requested count); JMAP uses `Email/set.updated`; the other
  backends error per id, so their success count equals the request.
  - Sub-fix in **io-imap**: MOVE's `COPYUID` arrives in an *untagged*
    `OK` (RFC 6851 §4.4), which the coroutine ignored — it only read the
    tagged reply — so every successful move reported `0`. `move.rs` now
    also scans untagged codes (+ a regression test). himalaya
    path-patches io-imap until a release >0.2.0 ships this.

- **F3 — inconsistent default-mailbox resolution across shared
  commands. FIXED.** `copy`/`move` used to hardcode
  `#[arg(default_value = "Inbox")]` on `--from`, silently selecting the
  literal INBOX, while `envelope list`/`search` required
  `mailbox.alias.inbox`. `--from` is now `Option<String>` resolved via
  the shared `resolve_mailbox_or_default`, so an omitted source falls
  back to the `inbox` alias and errors identically when none is
  configured. Every shared command now shares one policy: names come
  from the user or the alias map; the shared layer never guesses an
  inbox id.

### Provider-specific behaviour (not bugs)

- **P1 — Fastmail search is word/token-based, not substring.**
  `from ali`, `subject nline`, single letters `subject e` / `subject a`
  all return 0; whole words (`from alice`, `subject inline`,
  `subject Eta`) match. The trace shows himalaya issues the correct
  `SEARCH SUBJECT "e"` (inside a UID `SORT`); Fastmail's full-text index
  decides matching. Users expecting substring search will be surprised —
  worth documenting per provider (Dovecot-based servers may differ).

- **P2 — `date`/`after` target the `Date:` header (sent date), not
  server arrival.** Freshly-appended messages (all with today's
  INTERNALDATE) still filtered by their `Date:` header:
  `after 2026-07-16` returned only the 17th-dated messages,
  `date 2026-07-16` returned the single 16th-dated one. This makes the
  RFC 2822 date parser (and the wrong-weekday tolerance fix) load-bearing
  for search correctness.

- **P3 — omitting `-m` needs `mailbox.alias.inbox`.** By design there is
  no universal inbox default (JMAP's inbox is an opaque id), so the
  fallback requires an explicit alias; without it the error is clear and
  actionable. Set `mailbox.alias.inbox = "Inbox"` to use bare `-m`-less
  commands on IMAP.

### Observations

- **O1 — `message read --json`** emits the raw `mail_parser` `Message`
  structure (byte offsets, nested `parts`), which is low-level compared
  to the clean `envelope list --json` shape. Fine for machine use, but
  not an obvious public contract.
- `message read` uses `BODY.PEEK[]` — reading does not set `\Seen`.
- `attachment download` de-duplicates filename collisions on disk
  (`note.txt` → `note (1).txt`).
- `forward` quotes the original body with `> …` (like a reply) rather
  than a `-------- Forwarded message --------` block — a stylistic
  choice, flagged for awareness.
- Flag table symbols: `*` = unseen, `!` = flagged, blank = seen.

## Verdict

IMAP + SMTP on Fastmail is **solid and release-ready** for the shared
command surface: every command and flag works, output (table + JSON) is
correct, error handling is clean. No functional blocker. F1 (empty-input
validation), F2 (accurate copy/move counts, incl. the io-imap untagged
MOVE `COPYUID` sub-fix) and F3 (default-mailbox consistency) are all
**fixed**. P1/P2/P3 are provider/design behaviours to document, not fix.
