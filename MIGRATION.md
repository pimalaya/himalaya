# Migration guide

## From v2 to v3

v3 makes Himalaya a low-level CLI: plain text for a human to glance at, JSON for scripts and frontends. Daily mail is a frontend's job: see the interfaces listed in README.md. v3 drops the configuration wizard, takes one configuration path per `-c` and prints plain text, with no table nor colour. A v2 configuration loads as it is, unless it sets a colour key under `mailbox`, sets `envelope.list.datetime-relative` or marks several accounts default.

What a script or a frontend reads off `--json` and the text output changes, and a few commands that used to report a success they did not achieve now fail. The CHANGELOG lists every change; these are the ones to act on.

### Configuration and first run

- **The wizard is gone.** Bare `himalaya` prints the help, `himalaya configure` (alias `wizard`) and its `himalaya-configure` schema no longer exist, and no command offers to create a configuration. Write it from config.sample.toml, which releases and packages ship as share/doc/himalaya/config.sample.toml: run at a terminal without a configuration, Himalaya prints the commands copying it in place. Drop the `wizard` cargo feature from a build's `--features`.
- **`-c` takes one path per flag.** A value is no longer split on `:`, so a Windows path stays whole. Repeat the flag to merge several files: `-c public.toml -c private.toml`.
- **`HIMALAYA_CONFIG` is no longer read.** Pass `-c "$HIMALAYA_CONFIG"` for one path, and one `-c` per path for what was a `:`-separated list.
- **Every default path is read, as layers.** $XDG_CONFIG_HOME/himalaya/config.toml, $HOME/.config/himalaya/config.toml and $HOME/.himalayarc are each deep-merged on top of the ones before, where v2 read the first one found. An rc file left beside a config.toml now overrides it: delete the one you do not use.
- **Several `default = true` accounts are refused**, naming them, where v2 picked one at random. Keep it on one account across the merged files: an rc file moving the default sets `default = false` on the account the other file marks.
- **Colour, table and glyph keys are gone.** Remove the `*-color` keys of `envelope.list.table`, `mailbox.list.table`, `attachment.list.table` and `account.list.table`, `table.preset` and `table.arrangement`, and the `envelope.list.table.*-char` glyphs. Under `mailbox` they fail the configuration as unknown keys; elsewhere they are ignored, the whole `envelope.list.table` block included. The envelope lists mark a message's state with fixed characters, `*` unread, `R` answered, `!` flagged and `@` attachment.
- **`envelope.list.datetime-relative` is gone**, refused as an unknown key: remove it. Dates print as `envelope.list.datetime-fmt` and `datetime-local-tz` configure them.
- **Dates default to `%F %R` in the local timezone**, where v2 printed `%F %R%:z` at the offset a message was sent with. Set `envelope.list.datetime-fmt = "%F %R%:z"` and `envelope.list.datetime-local-tz = false` to keep the v2 dates.
- **`http` and `https` hosts are ASCII.** A JMAP `server` or an HTTP proxy `url` with an internationalised host is refused: write the host in Punycode (`https://xn--bcher-kva.example` for `bücher.example`). IMAP, SMTP and ManageSieve servers are unchanged.

### Text output

- **Every list prints one item per line, the way `mscan` prints a mailbox**, in columns two spaces apart with no header row, no border, no colour and no truncation. A message prints its id, four flag columns (`*` unread, `R` answered, `!` flagged, `@` attachment), its date, its senders' names, or `to ` and the recipients' with `--recipient`, then its subject: `envelope list|search`, `jmap email query|get`, `msgraph messages list`, `pimdir search`, and `imap thread` and `pimdir thread`, a reply's subject indented. The size no longer prints. Every other list prints its short cells first and its free text last (`mailbox list` ends with the name), a multi-valued cell joining its values with `,`, a yes-or-no cell its word (`default`, `active`, `permanent`) where v2 printed `yes` or `true`, and a mailbox's unread and total counts one cell (`3/120`) where v2 printed `TOTAL` and `UNREAD` columns. A script cutting the v2 tables or the column labels reads `--json` instead, which is unchanged.
- **Every list has a header**: `Account:`, what is listed (`Mailbox:`, `Message:`, a search's `Query:`) and, on a paged list, `Page:` and `Next page:` (`--page` or `--page-token`), then a blank line and the items, nothing under them. A script cutting the lines skips everything up to the first blank line (`sed '1,/^$/d'`), or reads `--json` with jq:

  ```sh
  # id, sender and subject, one envelope per line
  himalaya envelope list --json | jq -r '.envelopes[] | [.id, .from[0].email, .subject] | @tsv'
  # the ids of the unread envelopes
  himalaya envelope list --json | jq -r '.envelopes[] | select(all(.flags[]; .iana != "seen")) | .id'
  # the ids of the envelopes whose subject mentions an invoice
  himalaya envelope list --json | jq -r '.envelopes[] | select(.subject | test("invoice"; "i")) | .id'
  ```
- **Records are `Label: value` lines**: `account check`, `imap select`, `imap status`, `imap id`, `imap fetch`, `vacation get|set`, `jmap vacation get`, `sieve capability`, the Gmail and Microsoft Graph `get`s, and the header block of `message read` and `message parse`. `imap search` and `imap sort` no longer end with a count.
- **`-w/--max-width` is gone** from `envelope list`, `envelope search`, `mailbox list` and the pimdir lists, nothing being truncated any more: a command passing it now fails, so drop it.
- **`--help` prints without colour**, each paragraph on one line for the terminal to fold.

### JSON output

- **Keys are camelCase.** `messages-total` is `messagesTotal`, `next_page` is `nextPage`, `thread-id` is `threadId`. Values keep their spelling: error codes, flags and mailbox roles stay kebab-case or lowercase, provider passthrough keeps the provider's keys.
- **`message read --json` is a designed view**, the same on every backend (`headers`, `text`, `html`, `parts`), no longer mail-parser's model of the message. `message parse` prints the same view.
- **Piped `--json` prints one value per line**, and every schema `json-schema` writes carries an `$id` naming the contract version.
- **A failure carries a stable `code`** when a caller is expected to act on it (`body-pending`, `search-unsupported`, `capability-missing`, the pimdir store codes), described by the `himalaya-error` schema, and `null` otherwise. Match on the code, never on the message, which may be reworded.
- **No output is a sentence any more.** The `{"message": "..."}` confirmations are gone: every write prints what it acted on. `message copy|move` print `{count, queued}`; `message send` and a composer's `--save`/`--send` print `{id, saved, sent, queued}`; a protocol write prints `{id}` or `{ids}` (a Gmail label's id, the UIDs of an IMAP append or copy, a Sieve script's name). A unified write's `queued` is `true` when a sync engine applies it later (pimdir).
- **Bytes never reach a JSON stdout raw.** `message read --raw`, `jmap email export`, the raw Gmail and Graph `get`s and their attachment downloads print `{data, size, mime}`, `data` in base64.
- **Absent values are `null`**, every key printed.
- **Shapes made one:** flags written print `{raw, iana}` like envelopes, `inReplyTo` is a list in the read view, `imap fetch` addresses are `{name, email}`, `imap search` ids are numbers, an attachment row carries `role` instead of `inline`, maildir and mbox `flag list` rows are `{raw, iana}`.
- **Listings say whether more follow.** `envelope list` and `search` print `nextPage`, `null` on the last page, which `--page-token` takes back; `msgraph messages list` does the same with Graph's `$skip`.
- **`account check` prints every backend and exits 2** when one fails, each row carrying the failure's `code`, where v2 exited 0 whatever failed.
- **A protocol command prints its completion.** Every `imap` command prints `status` (`OK`, or `NO`/`BAD` with `text`, exiting 2). `imap raw` and `sieve raw` print the server's untagged or data responses as events, one JSON line each, then the completion; `smtp raw` prints `{code, text}`. Exit 2 means the server refused; exit 1 still means no answer.
- **pimdir names a queue row `row`** (`queueId`, `copyQueueId` and the queue view's `id` before); `pimdir flag add|set|remove` print `{messages: [{id, row}]}`; tag statuses are kebab-case (`unknown-flags`, `ambiguous-source`).
- **Gmail lists use the provider's key**: `messages list` prints `messages` (not `ids`), the settings lists `delegates`, `filter` and `forwardingAddresses` (not `response`); settings writes print the settings.

### Commands that now fail instead of pretending

- **An unserved verb fails up front with `capability-missing`.** Each backend declares what it serves of the unified API, which `account capability` lists and the `Backends:` footer of `--help` shows. On Gmail and Microsoft Graph, `flag add|set|remove answered|draft` reported the messages flagged and changed nothing: it now fails. `message add` on Graph, and `--save` there, fail before connecting rather than after the send.
- **A command given ids none of which exists fails** (exit 1): `flag add|set|remove`, `message move`, `message copy` and `message delete`.
- **A search clause a backend cannot answer fails** with `search-unsupported`, quoting the clause, rather than being dropped; a query that does not parse fails with `search-invalid`.
- **An IMAP keyword a mailbox would not keep fails** with `capability-missing` when the mailbox's `PERMANENTFLAGS` lists neither it nor `\*`, rather than being stored for the session and lost on the next one. `message forward --send` still only warns when its `$Forwarded` is refused.

### Search results that change

- **Gmail dates are UTC days.** `after 2026-09-08` used to keep that day's mail and `date 2026-09-08` two days, Gmail reading a bare date as midnight in PST.
- **JMAP date clauses under `not` or `or`** read the server's received date; `not after 2026-01-01` used to list nothing.

### New

- `himalaya repl` runs commands over connections kept for the session, and `himalaya mcp` serves the unified API to MCP clients, read-only unless `--allow-write` or `--allow-send` is given.
- Unified verbs that used to be protocol-only: `mailbox create|rename|delete`, the `important` and `junk` flags, and `vacation get|set`, which reads the auto-reply from JMAP or Gmail whatever backend stores the mail. `himalaya jmap`, `gmail` and the other namespaces keep their own commands.

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

- **Deep refactor on top of the I/O-free pattern.** No async in the CLI, no backend abstraction. Both the binary and the underlying libraries are simpler.
- **Thinner shared API.** Just enough surface for interfaces (TUI, plugins) to drive any backend, kept deliberately small so it does not break on every release.
- **Protocol-specific commands** (`himalaya imap/jmap/maildir/smtp …`) expose the full native capability of each protocol.
- **OAuth moved out** to [pimalaya/ortie](https://github.com/pimalaya/ortie).
- **Keyring moved out** to any password manager exposed as a shell command (`pass`, `secret-tool`, `gopass`, `security`, …).
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
- `list -f|--folder INBOX` becomes `list -m|--mailbox INBOX`. The flag is optional: when omitted, the inbox is used.
- The v1 search query grammar drops the `before <date>` clause. The remaining operators (`and`, `or`, `not`, parens) and the sort suffix (`order by date|from|to|subject [asc|desc]`) are unchanged. Backends advertise the subset they accept, and unsupported clauses fail at parse time. It is now accessible from the `search` command instead of `list`.
- Default page size moves to `envelope.list.page-size` (per-account, with global fallback). The `-s/--page-size` CLI flag still wins when passed. Hard fallback when neither is set: 25.

#### Flags

- `--folder` becomes `-m|--mailbox <NAME>` (optional, same default as `envelopes list`).
- `<id-or-flags>` split into `-f`, `--flag <FLAG>` (repeatable) and a positional `<message-ids>`.

#### Messages

- Removed `delete`: too protocol-specific. Use the matching protocol-specific subcommand, or combine `flags add` with the per-protocol expunge / move-to-trash step.
- `copy` and `move`: `--folder <source>` renamed `--from <mailbox-id>`, and positional `<target>` renamed `--to <mailbox-id>`.
- `save` renamed `add` (kept as an alias, so `save` still works).
- `save --folder` (optional) becomes `add --mailbox` (mandatory).
- The old `save --file <PATH>` / positional split is gone: `add` now takes a single `MessageArg` (a positional file path, an inline raw value, or piped stdin).
- Added `add --flag` to attach flags at insertion time, and `add --send` (alias `save --send`) to push the message through the account's send path after the append.
- `write` renamed `compose` (kept as an alias). `compose`, `reply`, `forward` are no longer interactive. They build the message from CLI flags through the built-in flag composer. Interactive composition is delegated to standalone tools chained into `messages send` / `messages add` via a tempfile or shell process substitution. No `*-with` subcommands or `[message.composer.*]` table remain on the himalaya side.
- `read` no longer renders human-readable text. The v2 `read` prints message-level info. For custom rendering, pipe `read --raw` into a standalone interpreter.
- `mailto:` URI handling is no longer a himalaya subcommand. Register a small shell wrapper (e.g. `mml mailto "$1" --output /tmp/draft.eml && himalaya messages send /tmp/draft.eml`) as your desktop mailto handler.
- `messages send` and `messages add` read the raw message from a positional path, an inline raw value, or stdin (the unified `MessageArg`).
- `export` and `edit` are removed.

See [pimalaya/mml](https://github.com/pimalaya/mml) for a ready-to-use composer / interpreter.

#### Attachments

- `download --folder` becomes `-m|--mailbox <NAME>` (optional, same default as `envelopes list`).
- `--downloads-dir` renamed `--dir`.
- Added an optional `<attachment-id>` positional to `download` (omit to download every attachment, preserving the v1 behaviour).
- Added a `list` subcommand.

#### Template

Fully removed. The template pipeline (compose / reply / forward drafts, MML compile, MIME interpret) lives in [pimalaya/mml](https://github.com/pimalaya/mml) as both a library and a CLI. Chain its CLI into `messages send` / `messages add` (see the README).

### Configuration changes

The full configuration schema is documented in [config.sample.toml](./config.sample.toml). The notes below focus on what changed since v1.

#### Global and per-account options

- `display-name`, `signature`, `signature-delim` and `downloads-dir` keep their v1 meaning, except that `signature` is the signature text only: a v1 file path is no longer read, pass `--signature-file` instead.
- The `template` and `pgp` top-level entries are removed. Rendering and PGP happen outside himalaya now (see the README for the recommended shell-pipeline shapes).
- The `message` table only reads `message.send.save-copy`, `message.reply.posting-style` and `message.forward.posting-style`; the rest of a v1 `[message]` table is ignored rather than rejected.
- Rename `template.reply.posting-style` → `message.reply.posting-style` and `template.forward.posting-style` → `message.forward.posting-style`, taking `top`, `bottom` or `none`. v1's `interleaved` is gone: use `none` and write the reply inside the quoted block yourself. v1's forward `attached` has no v2 equivalent.

#### Table customization

v3 removes these keys again: see [From v2 to v3](#from-v2-to-v3).

- The per-type `{account,folder,envelope}.list.table.{preset,arrangement}` keys collapse into a single `table.{preset,arrangement}` (global / per-account). `table.arrangement` (`dynamic`, `dynamic-full-width`, `disabled`) is new.
- Rename `folder.list.table.*` → `mailbox.list.table.*` (mirrors the `folders` → `mailboxes` command rename).
- Rename `envelope.list.table.sender-color` → `envelope.list.table.from-color` (the column is now `FROM`).

#### Mailbox aliases

The v1 `[folder.alias]` block becomes `[mailbox.alias]`. Two behaviour changes on top of the rename:

- Alias names are case-insensitive both on lookup and on storage, so `INBOX = "..."`, `Inbox = "..."` and `inbox = "..."` are equivalent entries.
- An entry named after a mailbox role (`inbox`, `sent`, `trash`...) overrides the role the backend reports. The `inbox` one is the default mailbox shared commands fall back to when `-m/--mailbox` is omitted. No separate `default-mailbox` key.

Account-level `[accounts.<name>.mailbox.alias]` entries override same-named global `[mailbox.alias]` entries.

#### Secrets

Every `*.passwd` / `*.password` / `*.token` field accepts either a raw literal (`{ raw = "…" }`) or a shell command (`{ command = "pass show foo" }` or `{ command = ["pass", "show", "foo"] }`). Native keyring support has been removed. Use a password-manager CLI (`pass`, `secret-tool`, `gopass`, `security`, …) as the command. OAuth tokens are produced by an external broker such as [pimalaya/ortie](https://github.com/pimalaya/ortie) and consumed the same way.

#### IMAP

The whole `backend.type = "imap"` block collapses into:

```toml
# Either a bare authority (treated as `imaps://<authority>`) or a full
# URL with `imap://` or `imaps://`. Mirrors `jmap.server`. A
# `unix:///path` scheme connects through a pre-authenticated session
# proxy such as sirup, over which no SASL is negotiated.
imap.server = "example.com"
# or imap.server = "imaps://example.com:993"
# or imap.server = "imap://example.com:143"  (use imap.starttls = true to upgrade)
# or imap.server = "unix:///run/user/1000/sirup/example-imap.sock"

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
imap.sasl.plain.passwd.command = ["pass", "show", "example"]

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

The OAuth-specific section (`backend.auth.type = "oauth2"`) is gone. Route the access token through SASL `oauthbearer` or `xoauth2` (with a command-sourced token from a broker such as [pimalaya/ortie](https://github.com/pimalaya/ortie)) instead.

#### SMTP

Same shape as IMAP, rooted at `[smtp]`. Bare authority defaults to `smtps://`. The v1 `message.send.backend.type = "smtp"` block becomes `smtp.server`, `smtp.tls.*`, `smtp.starttls`, `smtp.sasl.*` with the same SASL variants as IMAP.

#### Maildir

```toml
maildir.root = "~/Mail/example"
```

#### m2dir (new)

An [m2dir](https://man.sr.ht/~bitfehler/m2dir/) store: content-addressed messages with sidecar flag metadata.

```toml
m2dir.root = "~/Mail/example"
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
jmap.auth.bearer.token.command = ["pass", "show", "fastmail-api"]

# HTTP Basic
jmap.auth.basic.username = "user@example.com"
jmap.auth.basic.password.raw = "***"
# or
jmap.auth.basic.password.command = "pass show fastmail"

# Optional for `messages send` over JMAP: auto-discovered when omitted.
# Set them to pin a specific identity / drafts mailbox.
jmap.identity-id = "I0123abc"
jmap.drafts-mailbox-id = "M0123abc"
```

#### Gmail REST API (new)

The native Gmail backend authenticates with a single OAuth 2.0 bearer token from a broker such as [pimalaya/ortie](https://github.com/pimalaya/ortie). Labels are the mailboxes, addressed by name.

```toml
gmail.auth.token.command = ["ortie", "token", "show", "-a", "gmail"]
# or gmail.auth.token.raw = "***"
```

#### Microsoft Graph (new)

Same shape as Gmail: a single OAuth 2.0 bearer token. Sending goes through Graph, so no `smtp` block is needed.

```toml
msgraph.auth.token.command = ["ortie", "token", "show", "-a", "msgraph"]
```

#### Notmuch / Sendmail

Both backends are removed. Notmuch may come back in a future release.

### Suggested migration steps

1. Copy [config.sample.toml](./config.sample.toml) to a side-by-side path (for example ~/.config/himalaya/config.v2.toml) and edit it against your previous configuration.
2. Run `himalaya -c ~/.config/himalaya/config.v2.toml account check` to validate the connection for each declared backend.
3. Once the new file passes the check, replace the v1 config.toml with it.
4. If you relied on keyring / OAuth, wire a password-manager CLI (`pass`, `secret-tool`, …) and/or [pimalaya/ortie](https://github.com/pimalaya/ortie) as `command = …` secrets.
5. If you relied on the interactive `write` / `reply` / `forward`, install [pimalaya/mml](https://github.com/pimalaya/mml) and chain it into `himalaya messages send` / `messages add` via a tempfile or `>(...)` process substitution (see the README for ready-made `bash`/`zsh` snippets).
