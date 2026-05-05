# Himalaya CLI v2 — roadmap

Living document. Items are grouped by readiness and ordered roughly by
easiest-first within each group. Decisions land inline as `Decided:`
notes; open questions stay in section E until resolved.

## A. Shared API — already adequate

| Command | Status |
|---|---|
| `mailboxes list` | done |
| `envelopes list` | done (pagination: `--page` / `--page-size`, default size 25) |
| `flags add/set/delete` | done |
| `messages get` | done (`--raw` writes original RFC 5322 to stdout; `--json` emits the parsed struct) |
| `messages copy` | done (`<id>… --from <mailbox> --to <mailbox>`) |
| `messages move` | done (`<id>… --from <mailbox> --to <mailbox>`) |
| `messages add` | done (raw RFC 5322 in; `--mailbox <NAME>` + optional `--flag` / `--file`; IMAP APPEND, JMAP Blob/upload + Email/import, Maildir tmp-then-rename) |
| `messages compose` | done (CLI args incl. `--reply` / `--forward` with `--posting-style top\|bottom`, `--quote-headline`, `--signature`/`--signature-file`, `--send`) |
| `messages send` | done (raw RFC 5322 in; SMTP + JMAP) |
| `attachments list` | done (filename / mime / size / inline; `--include-inline` to surface inline parts) |
| `attachments download` | done (`--dir <PATH>`; defaults to account/global `downloads-dir`, then platform downloads dir, then temp) |

**Shared-command rule**: cross-backend commands always treat the IMAP
id as a UID. There is **no** `--seq` flag on the shared API — that
flag is reserved for the protocol-specific `imap` subcommands.

## B. Shared API — follow-ups (none currently)

`messages compose` no longer needs body quoting (shipped via
`--posting-style` + `--quote-headline` + `--signature`). `--save-to`
remains intentionally out of scope: the canonical pipeline is
`messages compose <args> | messages add --mailbox Drafts --flag draft`,
which composes well precisely because both halves are shared
commands. MIME-part extraction stays delegated to `mml`.


## C. Binary surface — to add back later

### C1. Sendmail / command backend

- Generic "exec a command, write the message to its stdin" backend.
  Replaces both v1's `sendmail` variant and any future `mailcmd`
  integrations.
- Config: `command.send = "/usr/sbin/sendmail -t"` (or similar).
- Wires only into `messages send`.

### C2. Notmuch backend

- Local index over Maildir. Adds `notmuch envelopes search` (real
  query support — Notmuch's reason to exist) and reuses the maildir
  arms for everything else.

### C3. `accounts` subcommand + wizard

- **Decided (E6)**: top-level verb is `accounts configure`.
- `accounts list` (read TOML, dump JSON).
- `accounts configure [<name>]` → wizard. Depends on the new
  `pimalaya/inquire` lib (crossterm-based, extracted from old
  `pimalaya-tui`) — himalaya hosts the `accounts configure` entry
  point, the lib provides the prompts.
- Possibly `accounts doctor` later (connectivity check).

### C4. Keyring support (opt-in)

- Cargo feature `keyring`, off by default. When on, secret values in
  TOML can be `{ keyring = "service:account" }`.
- Decoupled from auth itself — works for IMAP password, SMTP password,
  JMAP bearer, etc.

## D. Out of scope (delegated)

| Concern | Lives in |
|---|---|
| OAuth2 token acquisition / refresh | `pimalaya/ortie` (CLI) |
| MML composition, signing, encryption | `pimalaya/mml` (CLI) |
| HTML rendering | downstream tool / TUI |
| Search query DSL across backends | per-protocol commands only |
| Cross-backend sync | future `pimalaya/sirup` (or similar) |

Dropped permanently from v2 binary: `template *`, `attachment download`
(use `mml` for MIME part extraction), `messages delete` (use a flag or
move-to-trash), cross-backend copy, mailto handler.

## E. Resolved design decisions

1. **Pagination default size** — 25, matching v1. (E1)
2. **Compose shape** — single `messages compose` command with
   mutually-exclusive `--reply <id>` / `--forward <id>` flags. (E2)
3. **Compose vs send** — `compose --send` is supported. No interactive
   editor → no reason to force a `compose | send` pipe. (E3)
4. **Copy/move arg shape** — `<id>… --from <mailbox> --to <mailbox>`,
   with `--from` defaulting to `Inbox` and `--to` mandatory. (E4)
5. **`messages get` mode flags** — only `--raw`. JSON output is the
   global `--json` flag's job. No `--attachment-dir`, no `--part`. (E5)
6. **Wizard placement** — `accounts configure`. (E6)
7. **Backend dispatch** — none. The shared command runs unless the
   user explicitly types the protocol verb (`himalaya imap …`,
   `himalaya jmap …`, etc.), in which case the protocol-specific CLI
   takes over. (E7)

## F. Suggested ordering

1. **B1** compose body-quoting + `--save-to` — finish what's missing.
2. **C1** command/sendmail backend — drops in once the SMTP arm is
   stable.
3. **C3 + C4** account wizard + keyring — both gated on the inquire
   lib being extracted.
4. **C2** notmuch — last, depends on having maildir arms solid.
