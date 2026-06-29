# Migration guide

## From v1 to v2

### v1 issues

- **Backend abstraction was overkill.** A unifying trait across IMAP/Maildir/Notmuch/Sendmail looked tidy on paper but was a maintenance tax in practice.
- **The shared API restricted every protocol.** Keeping a single surface across very different backends meant either lossy shortcuts or confusing behaviour, and made adding a new backend risky.
- **OAuth was painful to configure and renew** (token refresh, retry, keyring round-trips, vendor quirks).
- **Native keyring integration was hazardous.** Platform-specific bugs, silent failures, locked sessions.
- **The configuration was verbose.** Deserialization errors were clear, but the per-account schema kept growing.
- **Composition was tangled with MML.** The boundary between "message" and "template" commands was unclear, and plugging a custom composer or reader was hard.
- **Native-TLS and Rustls did not coexist cleanly.**
- **Each command opened a fresh IMAP/SMTP session.** TCP, TLS, SASL, capability negotiation, every time.

#### v2 changes

- **Deep refactor on top of the I/O-free pattern.** No async in the CLI, no backend abstraction; both the binary and the underlying libraries are simpler.
- **Thinner shared API.** Just enough surface for interfaces (TUI, plugins) to drive any backend, kept deliberately small so it does not break on every release.
- **Protocol-specific commands** (`himalaya imap/jmap/maildir/smtp …`) expose the full native capability of each protocol.
- **OAuth moved out** to [pimalaya/ortie](https://github.com/pimalaya/ortie).
- **Keyring moved out** to [pimalaya/mimosa](https://github.com/pimalaya/mimosa) (or any password manager exposed as a shell command).
- **Composition and reading moved out** to [pimalaya/mml](https://github.com/pimalaya/mml).
- **Session reuse moved out** to [pimalaya/sirup](https://github.com/pimalaya/sirup), which exposes a pre-authenticated IMAP/SMTP session over a Unix socket.

A direct consequence: the v2 binary is about three times smaller than v1!

### Foundations: I/O-free

Pimalaya has been working for the past year on an adaptation of the [Sans I/O](https://sans-io.readthedocs.io/) pattern for its libraries. The pattern decouples the protocol state machine from any specific I/O runtime: sync vs async, tokio vs async-std vs smol, rustls vs native-tls. The concept has been validated in [pimalaya/ortie](https://github.com/pimalaya/ortie), [pimalaya/cardamum](https://github.com/pimalaya/cardamum) and [pimalaya/calendula](https://github.com/pimalaya/calendula), and is now wired into Himalaya CLI v2.

As a direct consequence, TLS is selectable at build time between `native-tls` and `rustls` (with `aws-lc` or `ring` as the crypto provider).

### CLI changes

#### Global flags

| v1 | v2 |
|---|---|
| `-o`, `--output {plain,json}` | `--json` only |
| `--quiet` / `--debug` / `--trace` | `--log-level {off,error,warn,info,debug,trace}` (alias `--log`) |
| `-f`, `--folder` | `-m`, `--mailbox` |

New in v2: `-b`, `--backend` (force a specific backend for shared commands) and `--log-file <PATH>` (write logs straight to a file).

#### Folders

- Renamed `folders` to `mailboxes`.
- Removed `add`, `expunge`, `purge`, `delete`: these are rarely useful at the interface level (Emacs, Vim plugin, TUI). Use the protocol-specific subcommands instead (`himalaya imap create`, `himalaya imap expunge`, `himalaya imap delete`).
- Added `--counts` to `list` to populate per-mailbox message counts.

#### Envelopes

- `thread` moved to the protocol-specific APIs.
- `list -f|--folder INBOX` becomes `list -m|--mailbox INBOX`. The flag is optional: when omitted, the id mapped to the `inbox` alias under `[mailbox.alias]` is used.
- The v1 search query grammar drops the `before <date>` clause. The remaining operators (`and`, `or`, `not`, parens) and the sort suffix (`order by date|from|to|subject [asc|desc]`) are unchanged. Backends advertise the subset they accept; unsupported clauses fail at parse time. It is now accessible from the `search` command instead of `list`.
- Default page size moves to `envelope.list.page-size` (per-account, with global fallback); the `-s/--page-size` CLI flag still wins when passed. Hard fallback when neither is set: 25.

#### Flags

- `--folder` becomes `-m|--mailbox <NAME>` (optional, same default as `envelopes list`).
- `<id-or-flags>` split into `-f`, `--flag <FLAG>` (repeatable) and a positional `<message-ids>`.

#### Messages

- Removed `delete`: too protocol-specific. Use the matching protocol-specific subcommand, or combine `flags add` with the per-protocol expunge / move-to-trash step.
- `copy` and `move`: `--folder <source>` renamed `--from <mailbox-id>`; positional `<target>` renamed `--to <mailbox-id>`.
- `save` renamed `add` (kept as an alias, so `save` still works).
- `save --folder` (optional) becomes `add --mailbox` (mandatory).
- `save <path-or-raw>` split into the explicit `--file <PATH>` and positional `<raw>`.
- Added `add --flag` to attach flags at insertion time.
- `write`, `reply`, `forward` are no longer interactive. They build the message from CLI flags through the built-in flag composer. Interactive composition is delegated to standalone tools chained into `messages send` / `messages add` via a tempfile or shell process substitution; no `*-with` subcommands or `[message.composer.*]` table on the himalaya side.
- `read` no longer renders human-readable text; the v2 `read` prints message-level info. For custom rendering, pipe `read --raw` into a standalone interpreter.
- `mailto:` URI handling is no longer a himalaya subcommand. Register a small shell wrapper (e.g. `mml mailto "$1" /tmp/draft.eml && himalaya messages send /tmp/draft.eml`) as your desktop mailto handler.
- `messages send` and `messages add` read the raw message from a positional path, an inline raw value, or stdin (the unified `MessageArg`).
- `export` and `edit` are removed.

See [pimalaya/mml](https://github.com/pimalaya/mml) for a ready-to-use composer / interpreter.

#### Attachments

- `download --folder` becomes `-m|--mailbox <NAME>` (optional, same default as `envelopes list`).
- `--downloads-dir` renamed `--dir`.
- Added an optional `<attachment-id>` positional to `download` (omit to download every attachment, preserving the v1 behaviour).
- Added a `list` subcommand.

#### Template

Fully removed. The template pipeline (compose / reply / forward drafts, MML compile, MIME interpret) lives in [pimalaya/mml](https://github.com/pimalaya/mml) as both a library and a CLI; chain its CLI into `messages send` / `messages add` (see the README).

### Configuration changes

The full configuration schema is documented in [config.sample.toml](./config.sample.toml). The notes below focus on what changed since v1.

#### Global and per-account options

- Removed `display-name`, `signature`, `signature-delim`: composition left the CLI.
- Only `downloads-dir` remains for the `attachments download` command.
- The `message`, `template` and `pgp` top-level entries are removed. Composition and rendering happen outside himalaya now (see the README for the recommended shell-pipeline shapes).

#### Table customization

- The per-type `{account,folder,envelope}.list.table.{preset,arrangement}` keys collapse into a single `table.{preset,arrangement}` (global / per-account). `table.arrangement` (`dynamic`, `dynamic-full-width`, `disabled`) is new.
- Rename `folder.list.table.*` → `mailbox.list.table.*` (mirrors the `folders` → `mailboxes` command rename).
- Rename `envelope.list.table.sender-color` → `envelope.list.table.from-color` (the column is now `FROM`).

#### Mailbox aliases

The v1 `[folder.aliases]` block becomes `[mailbox.aliases]`. Two behaviour changes on top of the rename:

- Alias names are case-insensitive both on lookup and on storage, so `INBOX = "..."`, `Inbox = "..."` and `inbox = "..."` are equivalent entries.
- The entry named `inbox` (case-insensitive) is the implicit default mailbox: shared commands fall back to its id when `-m/--mailbox` is omitted. No separate `default-mailbox` key.

Account-level `[accounts.<name>.mailbox.alias]` entries override same-named global `[mailbox.alias]` entries.

#### Secrets

Every `*.passwd` / `*.password` / `*.token` field accepts either a raw literal (`{ raw = "…" }`) or a shell command (`{ command = "pass show foo" }` or `{ command = ["pass", "show", "foo"] }`). Native keyring support has been removed; use [pimalaya/mimosa](https://github.com/pimalaya/mimosa) (or `pass`, `secret-tool`, `gopass`…) as the command. OAuth tokens are produced by an external broker such as [pimalaya/ortie](https://github.com/pimalaya/ortie) and consumed the same way.

#### IMAP

The whole `backend.type = "imap"` block collapses into:

```toml
# Either a bare authority (treated as `imaps://<authority>`) or a full
# URL with `imap://` or `imaps://`. Mirrors `jmap.server`.
imap.server = "example.com"
# or imap.server = "imaps://example.com:993"
# or imap.server = "imap://example.com:143"  (use imap.starttls = true to upgrade)

imap.tls.provider = "rustls"     # or "native-tls"
imap.tls.rustls.crypto = "ring"  # or "aws"
imap.tls.cert = "/path/to/custom/cert.pem"

imap.starttls = false

# Pick exactly one SASL mechanism. Omit the whole `imap.sasl` table to
# skip authentication entirely.

# SASL ANONYMOUS
imap.sasl.anonymous.message = "himalaya"

# SASL PLAIN
imap.sasl.plain.authcid = "user@example.com"
imap.sasl.plain.passwd.raw = "***"
# or
imap.sasl.plain.passwd.command = ["mimosa", "password", "read", "example"]

# SASL LOGIN
imap.sasl.login.username = "user@example.com"
imap.sasl.login.password.raw = "***"

# SASL OAUTHBEARER (RFC 7628)
imap.sasl.oauthbearer.username = "user@example.com"
imap.sasl.oauthbearer.host = "imap.example.com"
imap.sasl.oauthbearer.port = 993
imap.sasl.oauthbearer.token.command = ["ortie", "token", "read", "example"]

# SASL XOAUTH2 (Google)
imap.sasl.xoauth2.username = "user@example.com"
imap.sasl.xoauth2.token.raw = "***"

# SASL SCRAM-SHA-256 (RFC 7677)
imap.sasl.scram-sha-256.username = "user@example.com"
imap.sasl.scram-sha-256.password.raw = "***"
```

The OAuth-specific section (`backend.auth.type = "oauth2"`) is gone; route the access token through SASL `oauthbearer` or `xoauth2` (with a command-sourced token from a broker such as [pimalaya/ortie](https://github.com/pimalaya/ortie)) instead.

#### SMTP

Same shape as IMAP, rooted at `[smtp]`. Bare authority defaults to `smtps://`. The v1 `message.send.backend.type = "smtp"` block becomes `smtp.server`, `smtp.tls.*`, `smtp.starttls`, `smtp.sasl.*` with the same SASL variants as IMAP.

#### Maildir

```toml
maildir.root = "~/Mail/example"
```

#### JMAP (new)

```toml
jmap.server = "fastmail.com"
# or
jmap.server = "https://api.fastmail.com/jmap/session"

jmap.tls.provider = "rustls"     # or "native-tls"
jmap.tls.rustls.crypto = "ring"  # or "aws"
jmap.tls.cert = "/path/to/custom/cert.pem"

# Pick exactly one of `header`, `bearer`, `basic`.

# Raw "Authorization" header value, used verbatim
jmap.auth.header.raw = "Bearer eyJhbGciOiJ..."
jmap.auth.header.command = "pass show fastmail-raw-token"

# OAuth 2.0 / API token bearer
jmap.auth.bearer.token.raw = "***"
# or
jmap.auth.bearer.token.command = ["mimosa", "password", "read", "fastmail-api"]

# HTTP Basic
jmap.auth.basic.username = "user@example.com"
jmap.auth.basic.password.raw = "***"
# or
jmap.auth.basic.password.command = "pass show fastmail"

# Required only for `messages send` over JMAP.
jmap.identity-id = "I0123abc"
jmap.drafts-mailbox-id = "M0123abc"
```

#### Notmuch / Sendmail

Both backends are removed. Notmuch may come back in a future release.

### Suggested migration steps

1. Copy [`config.sample.toml`](./config.sample.toml) to a side-by-side path (for example `~/.config/himalaya/config.v2.toml`) and edit it against your previous configuration.
2. Run `himalaya -c ~/.config/himalaya/config.v2.toml account check` to validate the connection for each declared backend.
3. Once the new file passes the check, replace the v1 `config.toml` with it.
4. If you relied on keyring / OAuth, install [pimalaya/mimosa](https://github.com/pimalaya/mimosa) and/or [pimalaya/ortie](https://github.com/pimalaya/ortie) and wire them as `command = …` secrets.
5. If you relied on the interactive `write` / `reply` / `forward`, install [pimalaya/mml](https://github.com/pimalaya/mml) and chain it into `himalaya messages send` / `messages add` via a tempfile or `>(...)` process substitution (see the README for ready-made `bash`/`zsh` snippets).
