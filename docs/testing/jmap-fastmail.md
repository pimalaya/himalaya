# JMAP on Fastmail — shared-command test report

Shared cross-protocol commands on the JMAP backend. Companion:
[jmap-specific-fastmail.md](jmap-specific-fastmail.md) (the `jmap …` raw
API). Method: [provider-test-plan.md](provider-test-plan.md), every
command × flag by hand inside two throwaway JMAP mailboxes
(`Himalaya-Jmap-A`/`-B`).

- himalaya: `v2.0.0-alpha.1 +jmap +rustls-ring` (working tree with the
  shared-JMAP name→id, blob-download, CRLF, F1/F2/F3 fixes)
- account: `fastmail-jmap` (`jmap.server = api.fastmail.com`, bearer
  token)
- date: 2026-07-17
- fixtures: 5 messages (Alpha…Epsilon) with distinct
  sender/subject/date/flags + one `multipart/mixed` with a `note.txt`
  attachment.

## Results

| Command | Variants tested | Result |
| --- | --- | --- |
| `mailbox list` | base, `--counts` (inline), `--json` | ✅ |
| `envelope list` | base, paging, `-r`, `--has-attachment`, `--json`, by-name **and** by-opaque-id, empty mbox | ✅ |
| `envelope search` | from/to/subject/body/flag/not, date/after, and/or/grouping, `order by … asc/desc`, `--json` | ✅ (see J1) |
| `message add` | inline-esc, inline-LF, stdin, `--json`, `-f`; empty → `Message is empty` | ✅ |
| `message read` | pretty, multipart, `--raw`, `--json`, bad id | ✅ (blob download OK) |
| `flag add/set/remove` | multi-flag, set-replaces, remove | ✅ |
| `message copy` / `move` | by name, F2 counts, bogus id | ✅ (J2) |
| `attachment list` / `download` | list, `-i`, `-d` | ✅ |
| `message compose` | from/to/cc/bcc/subject/body/body-file/attach/signature/`--save` | ✅ |
| `message reply` / `forward` | `--save` (threading, `Re:`/`Fwd:`, quote) | ✅ |
| `message send` / `compose --send` | JMAP submission path | ✅ (auto-discovers identity + drafts, J3) |

Every earlier shared-JMAP bug (name-as-`inMailbox`, import-by-name,
`message read` 302 redirect) stays fixed: all commands address mailboxes
by their display **name** and resolve to the opaque id transparently.

## Findings

### Behaviour / config (not bugs)

- **J1 — `envelope search` attached short flags land in the query.**
  `envelope search -m A -s20 order by date asc` fails with `cannot parse
  search emails query \`-s20\``: the trailing `[QUERY]...` positional
  swallows the attached `-s20`. The spaced `-s 20` and `--page-size 20`
  both work. Inconsistent with `envelope list` (no trailing positional),
  where `-s20` is fine — worth documenting, and a candidate for a clap
  tweak. Not a JMAP/logic bug: sort + pagination work with the spaced
  form.

- **J2 — copy/move counts, bogus ids.** F2 counts work on JMAP
  (`Email/set.updated`): `1 message successfully copied`, etc. A bogus id
  **errors** (`Email/set failed for: <id>`, via `bail_on_not_updated`)
  rather than the IMAP no-op — stricter, and fine.

- **J3 — JMAP send config is now auto-discovered. FIXED.** `message
  send` / `compose --send` used to fail with `JMAP \`identity_id\` is
  required to send` because this account sets only server + token. The
  JMAP backend now falls back when those config fields are omitted: the
  identity from the account's default (first `Identity/get`), the drafts
  mailbox from the `drafts`-role mailbox (`Mailbox/get`). Set
  `identity_id` / `drafts_mailbox_id` only to pin specific ones.
  Verified: `message send` and `compose --send` deliver from this
  minimal (server + token) account with no extra config. A clear error
  is still raised if the account genuinely has no identity or no
  `drafts`-role mailbox.

- Date semantics match the shared DSL: `date`/`after` filter `receivedAt`
  on the wire then re-check `sentAt` client-side, so results follow the
  `Date:` header (Gamma@16th matched `date 2026-07-16`, excluded from
  `after 2026-07-16`).

- JMAP query index is eventually consistent right after a burst of
  imports: a sort-only query returned stale (empty) results once, then
  self-corrected. Not reproducible after settling.

## Verdict

Shared commands on JMAP are **solid and release-ready**: every command
and flag works, addressing mailboxes by name via the id resolver, table
+ JSON output correct, errors clean. No functional blocker. J1 is a
minor CLI-ergonomics nit (left in docs); J3 is **fixed** — JMAP send now
works from a minimal server + token account by auto-discovering the
identity and drafts mailbox.
