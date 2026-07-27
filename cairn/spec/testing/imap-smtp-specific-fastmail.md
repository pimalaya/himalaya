# IMAP + SMTP *specific* API on Fastmail — test report

Companion to [imap-smtp-fastmail.md](imap-smtp-fastmail.md) (which covers
the shared commands). This one exercises the raw protocol commands under
`himalaya imap …` and `himalaya smtp …`.

- himalaya: `v2.0.0-alpha.1 +imap +smtp +rustls-ring` (rev `1dddbe8` +
  working-tree fixes)
- account: `fastmail`
- date: 2026-07-17
- method: every `imap`/`smtp` subcommand and flag, by hand, inside two
  throwaway mailboxes (`Himalaya-Imap-A` / `-B`), per
  [provider-test-plan.md](provider-test-plan.md).

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `imap id` | base, `-p KEY:VAL` | ✅ |
| `imap create` / `delete` | fake A/B lifecycle | ✅ |
| `imap rename` | `A→A2→A` | ✅ |
| `imap subscribe` / `unsubscribe` | fake A | ✅ |
| `imap list` | default (LSUB), `-A` (LIST), `-p PATTERN` | ✅ (B1) |
| `imap status` | `<MAILBOX>` | ✅ |
| `imap flags` | available flags + PERMANENT | ✅ |
| `imap select` | `<MAILBOX>` | ✅ |
| `imap close` / `unselect` | no selection | ✅ errors as expected (B2) |
| `imap append` | `-f` flags, inline/file/stdin | ✅ but see **G1** |
| `imap fetch` | `--envelope --structure --flags --internal-date --size`, `--seq` | ✅ |
| `imap search` | from/to/cc/bcc/subject/body/text, before/since/on, larger/smaller, seen/unseen/flagged/…, `--seq`, AND of criteria | ✅ (B3) |
| `imap sort` | `-S <key>`, `-r`, criteria | ✅ |
| `imap thread` | `-A references` | ✅ |
| `imap store` | `--action add`, `-f <raw-flag>`, `--seq` | ✅ but see **G1** |
| `imap copy` | UID, `--seq` | ✅ (F2 applies) |
| `imap move` | UID | ✅ (F2 applies) |
| `imap expunge` | `\Deleted` + `EXPUNGE` | ✅ |
| `imap raw` | `a1 NOOP`, `a1 CAPABILITY` (tagged, verbatim reply) | ✅ |
| `smtp send` | `--mail-from`, `--rcpt-to`, piped body | ✅ delivered |
| `smtp raw` | `NOOP` → `250` | ✅ |

## Findings

### Bugs / issues

- **G1 — `imap append`/`imap store` `-f` takes *raw* flag tokens, not
  the shared `seen|answered|flagged|draft` enum.** `imap append -f seen`
  stores a **keyword** `seen`, not the system flag `\Seen`; likewise
  `-f flagged` → keyword `flagged`. Consequences verified:
  `imap search --seen` (`SEARCH SEEN`) does **not** match those messages
  (`--unseen` returned them), and `fetch --flags` shows the bare
  keyword. The system flag requires the backslash form:
  `imap append -f '\Seen'` correctly sets `\Seen` and is then found by
  `--seen`. This is defensible for a "raw IMAP" surface, but it is a
  **footgun**: the flag *names* collide with the shared `-f` enum
  (`message add -f seen` → `\Seen`) while the *meaning* differs, and the
  `--help` listed no possible values and gave no warning. **Resolved by
  documentation** (per owner decision — it is deliberately the raw API,
  not the shared one): the `-f` help on `imap append` and `imap store`
  now states that flags are raw IMAP tokens, that a bare word becomes a
  keyword, that system flags need the backslash (`-f '\Seen'`), and
  points to the shared `message add`/`flag add` for the enum behaviour.

- **F2 (inherited) — raw `imap copy`/`move` of a non-existent UID
  reported success. FIXED at the shared layer** (see the shared report's
  F2), including the io-imap untagged-MOVE-`COPYUID` sub-fix. The raw
  `imap copy`/`move` still print the generic `Message(s) successfully …`,
  since that surface mirrors the protocol verbatim; the count-aware
  messaging lives on the shared `copy`/`move`.

### Provider / protocol behaviour (not bugs)

- **B1 — `imap list` defaults to `LSUB` (subscribed mailboxes only);
  `-A` switches to `LIST` (all).** A freshly `create`d mailbox is not
  auto-subscribed, so it is invisible to bare `imap list` until
  `imap subscribe`; `imap list -A` always shows it. The trace confirms
  `Lsub{…}` vs `List{…}`.
- **B2 — `imap close` / `imap unselect` error** `BAD Please select a
  mailbox first`. Each CLI invocation is its own connection with nothing
  selected, so these mid-session commands have no state to act on.
  Expected.
- **B3 — `imap search --before/--since/--on` use `INTERNALDATE`
  (arrival)**, whereas the *shared* `envelope search` `date`/`after`
  target the `Date:` header (sent date). The raw command mirrors RFC
  3501 `SINCE/ON/BEFORE`; the shared DSL deliberately maps to
  `SENTSINCE/SENTON/SENTBEFORE`. Same-day appends match `--since today`
  regardless of their `Date:` header.

### Observations

- `imap fetch` renders a readable per-message block (flags, internal
  date, size, envelope fields, and an indented MIME structure tree).
- `imap raw` prints the server's untagged + tagged responses verbatim —
  useful for probing extensions.
- `imap sort -S date` places a message with no parseable `Date:` first.

## Verdict

The raw IMAP + SMTP surface is **complete and works end-to-end** on
Fastmail: every subcommand and flag behaves per RFC. **G1** (raw-flag-
token footgun on `append`/`store`) is resolved by clearer `-f` help, and
the inherited **F2** is fixed at the shared layer (with the io-imap
untagged-MOVE-`COPYUID` sub-fix). B1–B3 are protocol semantics to
document. No functional blocker.
