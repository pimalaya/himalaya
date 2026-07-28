# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `message read` gained a `--seen` flag to mark the message as seen while reading it; without it the read stays non-mutating (the default since v2). Backends that offer a side-effecting fetch do it in a single round (IMAP switches `BODY.PEEK[]` to `BODY[]`); the others issue a separate flag update after the fetch (JMAP `Email/set`, Gmail `messages.modify`, Microsoft Graph `PATCH isRead`, Maildir/m2dir add the `S` flag).

### Fixed

- `message read --raw --json` no longer errors: the two combine, emitting the raw RFC 5322 bytes as a JSON `{ "message": "…" }` string (lossily decoded) instead of bailing with "`--raw` and `--json` cannot be combined".

### Changed

- `message read`'s plain-text output is now a concise reading view: a minimal header block (Date, From, To, Cc, Subject) followed by a per-part walk — one summary line per MIME part (`[ID] <type>[ — <filename>] (<size>)`) with that part's own `Content-*` headers, then the decoded contents of plain-text parts inlined; HTML and binary parts stay a summary, except an HTML-only mail (whose sole text part is a single HTML one) has its markup printed rather than nothing readable. `--raw` (full RFC 5322 bytes) and `--json` (full parsed message) are unchanged for verbose or machine consumption.

- The `ID` in `message read` and in `attachments list` / `attachments download` is now the MIME part's 1-based position in the message (matching between the two commands), so `message read` shows the same id you pass to `attachments download`. Attachment ids are therefore a sparse subset of the part positions (e.g. `1 5 9`) rather than a dense `1 2 3` sequence.

- The first-run wizard now opens with a welcome banner on stderr explaining what Himalaya is and what the wizard does, replacing the comment header that used to head the generated config.

- The wizard now offers to save the generated configuration to a file (defaulting to `$XDG_CONFIG_HOME/himalaya/config.toml`, creating parent directories and confirming before overwriting an existing file) instead of only printing it. When stdout is redirected (`himalaya > config.toml`) or in JSON mode it still prints straight to stdout, so redirects and scripts keep working.

- Bare `himalaya --account <NAME>` (an account but no subcommand) now shows the help instead of dropping into the account-creation wizard.

- The wizard now offers only the IMAP authentication mechanisms the server supports.

  Before prompting, it opens an unauthenticated connection, reads the server's CAPABILITY, and offers only the advertised SASL mechanisms, most preferred first and the legacy `LOGIN` command last. A server exposing no SASL AUTH and no LOGINDISABLED (a perdition-style proxy such as isae-supaero.fr) offers `LOGIN` alone, where the wizard previously defaulted to `AUTHENTICATE PLAIN` and failed. Both the discovered and the manually entered IMAP paths probe; the manual path no longer hardcodes PLAIN. On any probe failure the wizard logs the error and falls back to the full mechanism list. SMTP keeps its discovery-advertised list, since it negotiates auth over EHLO.

- The wizard's service discovery is now time-bounded, so a single unreachable endpoint (a firewalled port, a black-hole host) no longer stalls it for the operating-system connect timeout. Mechanisms still running at the deadline are abandoned and only what completed in time is offered.

- The manually configured wizard flow now offers to reuse the IMAP credentials for SMTP, like the discovered flow, so they are entered once. On accept it reuses the IMAP SASL and prompts only the SMTP endpoint (host seeded from the IMAP host, encryption, port); on decline it runs the full SMTP prompts.

- `imap raw` now sends a byte-verbatim batch of tagged commands.

  The command argument decodes literal `\r` / `\n` escapes into real CRLF, so a CRLF-separated batch (each command carrying its own tag) can be pipelined from the shell, e.g. `himalaya imap raw 'a1 SELECT INBOX\r\na2 SEARCH ALL\r\n'`. A trailing CRLF is appended when omitted, and the reply is read until every command is acknowledged (possibly out of order). `smtp raw` gains the same escape decoding but stays a single command line: it strips the trailing CRLF (io-smtp appends its own) and rejects batched input, since the exchange reads exactly one reply. Both accept the command via stdin.

## [2.0.0] - 2026-07-26

### Added

- Added Gmail REST API support: a `[gmail]` account backend behind the shared mail commands (`--backend gmail`), plus a protocol-specific `gmail` command exposing the full REST surface (profile, labels, messages, attachments, drafts, threads, history, settings). Authenticates with a single OAuth 2.0 bearer token (`gmail.auth.token.raw` / `.command`), the only authorization Gmail's REST API accepts.

- Added Microsoft Graph REST API support: a `[msgraph]` account backend behind the shared mail commands (`--backend msgraph`), plus a protocol-specific `msgraph` command exposing the Graph mail surface (profile, mail-folder, message, attachment). Mirrors `[gmail]`, authenticating with a single OAuth 2.0 bearer token.

- Restored the RFC 2971 `ID`-after-auth quirk as `imap.id.{auto, fields}`, replacing the v1.2.0 `imap.extensions.id.send-after-auth` flag dropped during the v2 migration.

- Brought the `m2dir` backend to CLI feature parity with `maildir` (messages, flags, envelopes). Mailbox `rename` and message `copy` / `move` still await io-m2dir support.

- Added `--save <MAILBOX>` to `messages send`, mirroring the flag on `messages compose` / `reply` / `forward`.

- Added `--send` to `messages add` (alias `messages save`), mirroring `messages send --save`.

- Added raw passthrough commands `imap raw <command>` and `smtp raw <command>`.

- The wizard now pre-fills `mailbox.alias.*` from the server, so a generated account has a working default mailbox (the `inbox` alias, which backs commands that omit `-m/--mailbox`) and known Sent/Drafts/Trash/… targets without hand-editing ids. JMAP reads the RFC 8621 mailbox roles live over the tested connection. Gmail and Microsoft Graph map their fixed system-label ids (`INBOX`, `SENT`, …) and well-known folder names (`inbox`, `sentitems`, …). IMAP pins the reserved `INBOX` only. The other special-use roles await LIST `RETURN (SPECIAL-USE)` support in io-imap (upstream imap-codec has none yet).

- Added the `json-schema <DIR>` command, which writes one JSON Schema file per structured-output command (`himalaya-<cmd>-<subcmd>.json`) describing its `--json` payload, so downstream tooling can validate and type Himalaya's machine output. Only the schemas for compiled-in backends are emitted ([#547]).

- The discovery wizard now falls back to the system DNS resolver (/etc/resolv.conf on unix, the network adapters on windows) before the Cloudflare default, making the resolution chain `HIMALAYA_DNS_RESOLVER` then system then `1.1.1.1`. This avoids leaking the email domain to a third-party resolver and works around networks that block the default.

- Added local socket-proxy support: `imap.server` / `smtp.server` now accept a `unix:///path` scheme to connect through a pre-authenticated session proxy such as [sirup](https://github.com/pimalaya/sirup). No SASL is negotiated over the socket (the session arrives already authenticated), and a single-session proxy can back the storage backend without Himalaya opening a second connection ([#264]).

### Changed

- Flattened the `imap` command tree into top-level verbs mirroring the protocol's flat command list (`select`, `create`, `append`, `store`, `fetch`, …), replacing the former `mailbox` / `envelope` / `message` / `flag` subgroups.

- Unified raw-message input across the `messages`, `imap`, `maildir`, `jmap`, `msgraph` and `smtp` send/add commands behind a single `MessageArg` (file path, inline raw, or piped stdin), and removed the legacy `--file` flag on `messages add`.

- Split the merged `Account` out of every client wrapper: subcommands now receive `account` and `client` as sibling arguments instead of reaching through `client.account`.

- **Breaking:** changed bare `himalaya` (no subcommand) to run the account wizard instead of listing the default account's envelopes. Himalaya is now a lower-level tool: envelope listing lives under its explicit commands, and the bare invocation is the natural entry point for setting up an account.

- Changed the wizard to print the generated account as a ready-to-save TOML document on stdout instead of writing it to disk, mirroring ortie. Redirect it into your config file yourself (e.g. `himalaya > ~/.config/himalaya/config.toml`). It runs on bare `himalaya` and is still proposed when a command finds no config, while a config that exists but lacks the requested account is now a hard error rather than a wizard trigger.

  The account's connection is tested before the config is printed, so a bad credential or endpoint stops the wizard instead of yielding a config that cannot connect. The output is compact: only the `[accounts.<name>]` table stays a section header, every other table is flattened into dotted keys (`imap.sasl.plain.username = …`), and empty tables and defaulted values (`starttls`, `alpn`, `id.auto`) are dropped.

- Reworked the wizard's account-setup flow. The discovery list now shows one entry per reachable service (IMAP + SMTP, JMAP, Gmail, Microsoft Graph), and the authentication method is chosen in a second, service-specific prompt: the SASL mechanism (`PLAIN`, `LOGIN`, `SCRAM-SHA-256`, `OAUTHBEARER`, …) for IMAP + SMTP, the HTTP scheme (Basic / Bearer) for JMAP. OAuth 2.0 is no longer a dead-end list entry: it folds into the "API token" credential prompt, which now offers the OS keyrings and the OAuth token brokers (Ortie, pizauth, oama) together, the brokers appearing only when the service advertises OAuth.

- The wizard now tests IMAP and SMTP as it configures them: the IMAP connection is validated first, then it asks whether SMTP reuses the same credentials (the two may advertise different auth), re-running the SASL prompt for a distinct one, and tests SMTP last.

- The wizard no longer prompts for an account name. It is derived from the input (the domain's first label, or the folder name). Rename it by editing the printed `[accounts.<name>]` table key.

### Fixed

- Fixed compilation error when the `wizard` feature was disabled ([#634]).

- Fixed mailbox-alias resolution to apply across every shared `messages` subcommand (`add`, `read`, `reply`, `forward`, `copy`, `move`) and the `--save` flag on `compose` / `reply` / `forward`, so aliases like `mailbox.alias.sent` are honoured before the backend call.

- Fixed swapped save/send success messages ("saved" printed after a pure send, "sent" after a pure save).

### Removed

- Removed the `[message.composer.*]` / `[message.reader.*]` config tables and the `compose-with` / `reply-with` / `forward-with` / `mailto` / `read-with` subcommands. Richer composition now chains standalone tools such as [mml](https://github.com/pimalaya/mml) into `messages send` / `add`.

  The "stdout = MIME draft" contract was incompatible with composers spawning an interactive editor, whose UI inherited the parent's piped stdout.

- Dropped `HIMALAYA_CONFIG` environment-variable support. `-c/--config` still accepts one or more `:`-separated paths.

- Removed the `account configure` command (alias `account edit`) and its per-field edit wizard. Account setup now goes through the bare `himalaya` discovery wizard, leaving `account list` and `account check` for inspection.

## [1.2.0] - 2026-02-19

### Added

- Added `--quiet` flag ([#545]).
- Added support for custom TLS certificate.
- Added support for multiple config files via `HIMALAYA_CONFIG=path1:path2:…` ([#612]).
- Added `-d|--downloads-dir` argument to `attachment download` command ([#559]).
- Added `View` and `View MIME` post edit choices ([#486]).
- Made `message send` and `template send` return id ([#627]).
- Enabled shell expansion for email, IMAP and SMTP login fields, which allows usage of environment variables ([#632]).

### Fixed

- Bumped all deps, fixing `rustls-platform-verifier` compatibility version issue.
- Fixed invalid fetch with NIL body encoding ([#525]).
- Fixed freeze when sending large attachments ([#593]).
- Fixed abusive warn logs from IMAP libs ([#552]).
- Downgraded `toml@0.8` due to regression ([#611]).

## [1.1.0] - 2025-01-11

### Added

- Added `-y|--yes` flag for `folder purge` and `folder delete` commands ([#469]).

### Changed

- Put back `warn` the default log level ([#522]).

  Since logs are sent to `stderr`, warnings can be easily discarded by prepending commands with `RUST_LOG=off` or by appending commands with `2>/dev/null`.

- Changed `message.send.save-copy` default to `true` ([#536]).

- Changed default downloads directory. [core#1]

### Fixed

- Fixed permissions issues when using `install.sh` ([#515]).
- Fixed de/serialization issues of backends' `none` variant ([#523]).
- Fixed list envelopes out of bound error when empty result ([#535]).
- Fixed macOS x86_64 builds ([#538]).

## [1.0.0] - 2024-12-09

The Himalaya CLI scope has changed. It does not include anymore the synchronization, nor the envelope watching. These scopes have moved to dedicated projects:

- [Neverest CLI](https://github.com/pimalaya/neverest), CLI to synchronize, backup and restore emails
- [Mirador CLI](https://github.com/pimalaya/mirador), CLI to watch mailbox changes

Due to the long time difference with the previous `v1.0.0-beta.4` release, this changelog may be incomplete. The simplest way to upgrade is to reconfigure Himalaya CLI from scratch, using the wizard or the [`config.sample.toml`](./config.sample.toml).

Himalaya CLI will now try to adopt the [conventional commits specification](https://github.com/conventional-commits/conventionalcommits.org). Tools like [`git-cliff`](https://git-cliff.org/) may help us generating more accurate changelogs in the future.

### Added

- Added `message edit` command to edit a message. To edit on place (replace a message), use `--on-place`.
- Added `account.list.table.preset` global config option, `accounts.<name>.folder.list.table.preset` and `accounts.<name>.envelope.list.table.preset` account config options.

  These options customize the shape of tables, see examples at [`comfy_table::presets`](https://docs.rs/comfy-table/latest/comfy_table/presets/index.html). Defaults to `"||  |-|||           "`, which corresponds to [`comfy_table::presets::ASCII_MARKDOWN`](https://docs.rs/comfy-table/latest/comfy_table/presets/constant.ASCII_MARKDOWN.html).

- Added `account.list.table.name-color` config option to customize the color used for the accounts' `NAME` column (defaults to `green`).
- Added `account.list.table.backends-color` config option to customize the color used for the folders' `BACKENDS` column (defaults to `blue`).
- Added `account.list.table.default-color` config option to customize the color used for the folders' `DEFAULT` column (defaults to `reset`).
- Added `accounts.<name>.folder.list.table.name-color` account config option to customize the color used for the folders' `NAME` column (defaults to `blue`).
- Added `accounts.<name>.folder.list.table.desc-color` account config option to customize the color used for the folders' `DESC` column (defaults to `green`).
- Added `accounts.<name>.envelope.list.table.id-color` account config option to customize the color used for the envelopes' `ID` column (defaults to `red`).
- Added `accounts.<name>.envelope.list.table.flags-color` account config option to customize the color used for the envelopes' `FLAGS` column (defaults to `reset`).
- Added `accounts.<name>.envelope.list.table.subject-color` account config option to customize the color used for the envelopes' `SUBJECT` column (defaults to `green`).
- Added `accounts.<name>.envelope.list.table.sender-color` account config option to customize the color used for the envelopes' `FROM` column (defaults to `blue`).
- Added `accounts.<name>.envelope.list.table.date-color` account config option to customize the color used for the envelopes' `DATE` column (defaults to `dark_yellow`).
- Added `accounts.<name>.envelope.list.table.unseen-char` account config option to customize the char used for unseen envelopes (defaults to `*`).
- Added `accounts.<name>.envelope.list.table.replied-char` account config option to customize the char used for replied envelopes (defaults to `R`).
- Added `accounts.<name>.envelope.list.table.flagged-char` account config option to customize the char used for flagged envelopes (defaults to `!`).
- Added `accounts.<name>.envelope.list.table.attachment-char` account config option to customize the char used for envelopes with at least one attachment (defaults to `@`).

### Changed

- Refactored the `account configure` command: this command stands now for creating or editing account configurations from the wizard. The command requires the `wizard` cargo feature.
- Improved the `account doctor` command: it now checks the state of the config, and the new `--fix` argument allows you to configure keyring, OAuth 2.0 etc.
- Improved long version `--version` ([#496]).
- Improved error messages when missing cargo features. For example, if a TOML configuration uses the IMAP backend without the `imap` cargo features, the error `missing "imap" feature` is displayed ([#20]).(https://github.com/pimalaya/core/issues/20)
- Normalized enum-based configurations, using the [internally tagged representation](https://serde.rs/enum-representations.html#internally-tagged) `type =`. It should reduce issues due to misconfiguration, and improve othe error messages. Yet it is not perfect, se ([#802]).(https://github.com/toml-rs/toml/issues/802):

  - `imap.*`, `maildir.*` and `notmuch.*` moved to `backend.*`:

	```toml
	# before
	imap.host = "localhost"
	imap.port = 143

	# after
	backend.type = "imap"
	backend.host = "localhost"
	backend.port = 143
	```

  - `smtp.*` and `sendmail.*` moved to `message.send.backend.*`:

	```toml
	# before
	smtp.host = "localhost"
	smtp.port = 25

	# after
	message.send.backend.type = "smtp"
	message.send.backend.host = "localhost"
	message.send.backend.port = 25
	```

  - `pgp.backend` renamed `pgp.type`:

	```toml
	# before
	pgp.backend = "commands"
	pgp.encrypt-cmd = "gpg --encrypt --quiet --armor <recipients>"

	# after
	pgp.type = "commands"
	pgp.encrypt-cmd = "gpg --encrypt --quiet --armor <recipients>"
	```

  - `{imap,smtp}.auth` moved as well:

    ```toml
    # before
    imap.password.cmd = "pass show example"
    smtp.oauth2.method = "xoauth2"

    # after
    backend.auth.type = "password"
    backend.auth.cmd = "pass show example"
    message.send.backend.auth.type = "oauth2"
    message.send.backend.auth.method = "xoauth2"
    ```

- Moved IMAP and SMTP `encryption` to `encryption.type`.

  This change prepares the config to accept different TLS providers with their options. The `true` and `false` variant have been removed as well:

	```toml
	# before
	backend.encryption = "none" # or false
	backend.encryption = "start-tls"
	message.send.backend.encryption = "tls" # or true

	# after
	backend.encryption.type = "none"
	backend.encryption.type = "start-tls"
	message.send.backend.encryption.type = "tls"
	```

### Fixed

- Fixed pre-release archives issue ([#492]).
- Fixed mailto parsing issue. [core#10]
- Fixed `Answered` flag not set when replying to a message ([#508]).

### Removed

- Removed systemd service from `assets/` folder, as Himalaya CLI scope does not include synchronization nor watching anymore.

## [1.0.0-beta.4] - 2024-04-16

### Added

- Added systemd service in `assets/` folder.
- Added configuration option `message.delete.style` that can be either `folder` (deleted messages are moved to the Trash folder, default style) or `flag` (deleted messages receive the Deleted flag).
- Added `--debug` as an alias for `RUST_LOG=debug`.
- Added `--trace` as an alias for `RUST_LOG=trace` and `RUST_BACKTRACE=1`.
- Added notes about `--debug` and `--trace` when error occurs.

### Changed

- **Added back the search feature**: you can now give an optional filter and sort query at the end of the `envelope list` command. See `envelope list --help` or [pimalaya.org](https://pimalaya.org/himalaya/cli/master/usage/advanced/envelope/list.html#query) for more detail on the search API.
- Changed the `envelope list` folder argument due to the search query: it became a flag `--folder|-f`.
- Made the global `--config|-c` option repeatable: the first option is considered the path to the main config, and successive options are considered partial override ([#184]).
- Refactored error management: error should be more clear, colored and can now contain spantrace and backtrace.
- Made `--help` content wrapping properly thanks to the `clap` cargo feature `wrap_help`.
- Improved `template {new,reply,forward}` command JSON output: they return now a JSON object with 3 properties:
  - `content`: the content of the template
  - `cursor.row`: the row at which the cursor should be placed by the interface using the template
  - `cursor.col`: the column at which the cursor should be placed by the interface using the template

### Fixed

- Fixed watch IMAP envelopes when folder was empt ([#179]).
- Prevented parsing of undefined config option ([#188]).
- Fixed `In-Reply-To` header being skipped from mailto URL ([#194]).
- Fixed error page out of bounds when filtering envelopes returned an empty resul ([#195]).

## [1.0.0-beta.3] - 2024-02-25

### Added

- Added `account check-up` command.
- Added wizard warning about google password ([#41]).

### Changed

- Removed account configurations flatten level in order to improve diagnostic errors, due to a [bug](https://github.com/toml-rs/toml/issues/589#issuecomment-1872345017) in clap. **This means that accounts need to be prefixed by `accounts`: `[my-account]` becomes `[accounts.my-account]`**. It also opens doors for interface-specific configurations.
- Rolled back cargo feature additions from the previous release. It was a mistake: the amount of features was too big, the code (both CLI and lib) was too hard to maintain. Cargo features kept: `imap`, `maildir`, `notmuch`, `smtp`, `sendmail`, `account-sync`, `account-discovery`, `pgp-gpg`, `pgp-commands` and `pgp-native`.
- Moved `sync.strategy` to `folder.sync.filter`.
- Changed location of the synchronization data from `$XDG_DATA_HOME/himalaya/<account-name>` to `$XDG_DATA_HOME/pimalaya/email/sync/<account-name>-cache`.
- Changed location of the synchronization cache from `sync.dir` to `$XDG_CACHE_HOME/pimalaya/email/sync/<hash>/`.
- Replaced id mapping database `SQLite` by `sled`, a pure key-val store written in Rust to improve portability of the tool. **Therefore, id aliases are reset**.
- Improved pre and post edit choices interactio ([#58]).
- Improved account synchronization performances, making it 50% faster than `mbsync` and 370% faster than `OfflineIMAP`.
- Changed `envelope.watch.{event}.{hook}`: hooks can now be cumulated. For example it is possible to send a system notification and execute a shell command when receiving a new envelope:

  ```toml
  envelope.watch.received.notify.summary = "New message from {sender}"
  envelope.watch.received.notify.body = "{subject}"
  envelope.watch.received.cmd = "echo {id} >> /tmp/new-email-counter"
  ```

### Fixed

- Fixed bug that was preventing watch placeholders to be replaced when using shell command hook.
- Fixed watch IMAP envelopes issue preventing events to be triggered.
- Fixed DNS account discovery priority issues.
- Fixed SMTP messages not properly sent to all recipient ([#172]).
- Fixed backend feature badly linked, leading to reply and forward message error ([#173]).

## [1.0.0-beta.2] - 2024-01-27

### Added

- Added cargo feature `wizard`, enabled by default.
- Added one cargo feature per backend feature:
  - `account` including `account-configure`, `account-list`, `account-sync` and the `account-subcmd`
  - `folder` including `folder-add`, `folder-list`, `folder-expunge`, `folder-purge`, `folder-delete` and the `folder-subcmd`
  - `envelope` including `envelope-list`, `envelope-watch`, `envelope-get` and the `envelope-subcmd`
  - `flag` including `flag-add`, `flag-set`, `flag-remove` and the `flag-subcmd`
  - `message` including `message-read`, `message-write`, `message-mailto`, `message-reply`, `message-forward`, `message-copy`, `message-move`, `message-delete`, `message-save`, `message-send` and the `message-subcmd`
  - `attachment` including `attachment-download` and the `attachment-subcmd`
  - `template` including `template-write`, `template-reply`, `template-forward`, `template-save`, `template-send` and the `template-subcmd`
- Added wizard capability to autodetect IMAP and SMTP configurations, based on the [Thunderbird Autoconfiguration](https://wiki.mozilla.org/Thunderbird:Autoconfiguration) standard.
- Added back Notmuch backend features.

### Changed

- Renamed `folder create` to `folder add` in order to better match types. An alias has been set up, so both `create` and `add` still work.

### Fixed

- Fixed default command: running `himalaya` without argument lists envelopes, as it used to be in previous versions.
- Fixed bug when listing envelopes with `backend = "imap"`, `sync.enable = true` and `envelope.watch.backend = "imap"` led to unwanted IMAP connection creation (which slowed down the listing).
- Fixed builds related to enabled cargo features.

## [1.0.0-beta] - 2024-01-01

Few major concepts changed:

- The concept of *Backend* and *Sender* changed. The Sender does not exist anymore (it is now a backend feature). A Backend is now a set of features like add folders, list envelopes or send raw message. The backend of every single feature can be customized in the configuration file, which gives users more flexibility. Here the list of backend features that can be customized:
  - `backend` ***(required)***: the backend used by default by all backend features (`maildir`, `imap` or `notmuch`)
  - `folder.add.backend`: override the backend used for creating folders (`maildir`, `imap` or `notmuch`)
  - `folder.list.backend`: override the backend used for listing folders (`maildir`, `imap` or `notmuch`)
  - `folder.expunge.backend`: override the backend used for expunging folders (`maildir`, `imap` or `notmuch`)
  - `folder.purge.backend`: override the backend used for purging folders (`maildir`, `imap` or `notmuch`)
  - `folder.delete.backend`: override the backend used for deleting folders (`maildir`, `imap` or `notmuch`)
  - `envelope.list.backend`: override the backend used for listing envelopes (`maildir`, `imap` or `notmuch`)
  - `envelope.get.backend`: override the backend used for getting envelopes (`maildir`, `imap` or `notmuch`)
  - `envelope.watch.backend`: override the backend used for watching envelopes (`maildir`, `imap` or `notmuch`)
  - `flag.add.backend`: override the backend used for adding flags (`maildir`, `imap` or `notmuch`)
  - `flag.set.backend`: override the backend used for setting flags (`maildir`, `imap` or `notmuch`)
  - `flag.remove.backend`: override the backend used for removing flags (`maildir`, `imap` or `notmuch`)
  - `message.send.backend` ***(required)***: override the backend used for sending messages (`sendmail` or `smtp`)
  - `message.read.backend`: override the backend used for reading messages (`maildir`, `imap` or `notmuch`)
  - `message.write.backend`: override the backend used for adding flags (`maildir`, `imap` or `notmuch`)
  - `message.copy.backend`: override the backend used for copying messages (`maildir`, `imap` or `notmuch`)
  - `message.move.backend`: override the backend used for moving messages (`maildir`, `imap` or `notmuch`)
- The CLI API changed: every command is now prefixed by its domain following the format `himalaya <domain> <action>`. List of domain available by running `himalaya -h` and list of actions for a domain by running `himalaya <domain> -h`.
- TOML configuration file options use now the dot notation rather than the dash notation. For example, `folder-listing-page-size` became `folder.list.page-size`. See the [changed](#changed) section below for more details.

### Added

- Added cargo feature `maildir` (not plugged yet).
- Added cargo feature `sendmail` (not plugged yet).
- Added watch hooks `envelope.watch.received` (when a new envelope is received) and `envelope.watch.any` (for any other event related to envelopes). A watch hook can be:
  - A shell command: `envelope.watch.any.cmd = "mbsync -a"`
  - A system notification: 
    - `envelope.watch.received.notify.summary = "📬 New message from {sender}"`: customize the notification summary (title)
    - `envelope.watch.received.notify.body = "{subject}"`: customize the notification body (content)

	*Available placeholders: id, subject, sender, sender.name, sender.address, recipient, recipient.name, recipient.address.*
- Added watch support for Maildir backend features.

### Changed

- Renamed cargo feature `imap-backend` → `imap`.
- Renamed cargo feature `notmuch-backend` → `notmuch`.
- Renamed cargo feature `smtp-sender` → `smtp`.
- Changed the goal of the config option `backend`: it is now the default backend used for all backend features. Valid backends: `imap`, `maildir`, `notmuch`.
- Moved `folder-aliases` config option to `folder.alias(es)`.
- Moved `folder-listing-page-size` config option to `folder.list.page-size`.
- Moved `email-listing-page-size` config option to `envelope.list.page-size`.
- Moved `email-listing-datetime-fmt` config option to `envelope.list.datetime-fmt`.
- Moved `email-listing-datetime-local-tz` config option to `envelope.list.datetime-local-tz`.
- Moved `email-reading-headers` config option to `message.read.headers`.
- Moved `email-reading-format` config option to `message.read.format`.
- Moved `email-writing-headers` config option to `message.write.headers`.
- Move `email-sending-save-copy` config option to `message.send.save-copy`.
- Move `email-hooks.pre-send` config option to `message.send.pre-hook`.
- Moved `sync` config option to `sync.enable`.
- Moved `sync-dir` config option to `sync.dir`.
- Moved `sync-folders-strategy` config option to `sync.strategy`.
- Moved `maildir-*` config options to `maildir.*`.
- Moved `imap-*` config options to `imap.*`.
- Moved `notmuch-*` config options to `notmuch.*`.
- Moved `sendmail-*` config options to `sendmail.*`.
- Moved `smtp-*` config options to `smtp.*`.
- Replaced options `imap-ssl`, `imap-starttls` and `imap-insecure` by `imap.encryption`:
  - `imap.encryption = "tls" | true`: use required encryption (SSL/TLS)
  - `imap.encryption = "start-tls"`: use opportunistic encryption (StartTLS)
  - `imap.encryption = "none" | false`: do not use any encryption
- Replaced options `smtp-ssl`, `smtp-starttls` and `smtp-insecure` by `smtp.encryption`:
  - `smtp.encryption = "tls" | true`: use required encryption (SSL/TLS)
  - `smtp.encryption = "start-tls"`: use opportunistic encryption (StartTLS)
  - `smtp.encryption = "none" | false`: do not use any encryption

### Removed

- Disabled temporarily the `notmuch` backend because it needs to be refactored using the backend features system (it should be reimplemented soon).
- Disabled temporarily the `search` and `sort` command because they need to be refactored, se ([#39]).
- Removed the `notify` command (replaced by the new `watch` command).
- Removed all global options except for `display-name`, `signature`, `signature-delim` and `downloads-dir`.

## [0.9.0] - 2023-08-28

### Added

- Added 3 new cargo features:
  - `pgp-commands`: enables the commands PGP backend (enabled by default, same behaviour as before)
  - `pgp-gpg`: enables the GPG backend (requires the `gpgme` lib on the system)
  - `pgp-native`: enables the native PGP backend
- Added account configuration `pgp` to configure the way PGP operations are performed.

### Changed

- Moved `email-writing-encrypt-cmd`to `pgp.encrypt-cmd`.
- Moved `email-reading-decrypt-cmd` to `pgp-decrypt-cmd`.
- Moved `email-writing-sign-cmd` to `pgp.sign-cmd`.
- Moved `email-reading-verify-cmd` to `pgp.verify-cmd`.

## [0.8.4] - 2023-07-18

### Fixed

- Fixed windows releases due to cargo deps typo.

## [0.8.3] - 2023-07-18

### Fixed

- Fixed windows releases due to `coredump` crate compilation error.
- Fixed macos releases due to macos 12 System Integrity Protection.

## [0.8.2] - 2023-07-18

### Changed

- Made the code async using the tokio async runtime.
- On Linux, made the kernel keyring the default one (the one based on keyutils).

### Fixed

- Fixed the way folder aliases are resolved. In some case, aliases were resolved CLI side and lib side, which led to alias error ([#95]).

## [0.8.1] - 2023-06-15

### Added

- Implemented OAuth 2.0 refresh token flow for IMAP and SMTP, which means that access tokens are now automatically refreshed and is transparent for users.
- Added `imap-oauth2-redirect-host` and `smtp-oauth2-redirect-host` options to customize the redirect server host name (default: `localhost`).
- Added `imap-oauth2-redirect-port` and `smtp-oauth2-redirect-port` options to customize the redirect server port (default: `9999`).
- Added `email-listing-datetime-fmt` to customize envelopes datetime format. See format spec [here](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).
- Added `email-listing-local-datetime` to transform envelopes datetime's timezone to the user's local one. For example, if the user's local is set to `UTC`, the envelope date `2023-06-15T09:00:00+02:00` becomes `2023-06-15T07:00:00-00:00`.

### Fixed

- Fixed missing `<` and `>` around `Message-ID` and `In-Reply-To` headers.

## [0.8.0] - 2023-06-03

### Added

- Added keyring support, which means Himalaya can now use your system's global keyring to get/set sensitive data like passwords or tokens.
- Added required IMAP option `imap-auth` and SMTP option `smtp-auth`. Possible values: `passwd`, `oauth2`.
- Added OAuth 2.0 support for IMAP and SMTP.
- Added passwords and OAuth 2.0 configuration via the wizard.
- Added `email-sending-save-copy` option to control whenever a copy of any sent email should be saved in the `sent` folder defined in `folder-aliases`.
- Imported id mapper from the lib, which means that the id mapping is now done by the CLI.
- Added `BackendConfig` to `AccountConfig::backend` to match sender implementation.
- Added support for pipeline commands, which means commands can be either a single command (string) or piped commands (list of strings). It applies for:
  - `email-writing-verify-cmd`
  - `email-writing-decrypt-cmd`
  - `email-writing-sign-cmd`
  - `email-writing-encrypt-cmd`

### Changed

- Changed release archive extensions from `.tar.gz` to `.tgz`.
- Moved `wizard` module into domains (config, account, backend…).
- [**BREAKING**] Changed the way secrets are managed. A secret is a sensitive data like passwords or tokens. There is 3 possible ways to declare a secret in the config file:
  - `{ raw = <secret> }` for the raw secret as string (unsafe, not recommended),
  - `{ cmd = <secret-cmd> }` for command that exposes the secret,
  - `{ keyring = <secret-entry> }` for entry in your system's global keyring that contains the secret.

  This applies for:
    - `imap-passwd`
	- `imap-oauth2-client-secret`
	- `imap-oauth2-access-token`
	- `imap-oauth2-refresh-token`
    - `smtp-passwd`
	- `smtp-oauth2-client-secret`
	- `smtp-oauth2-access-token`
	- `smtp-oauth2-refresh-token`

### Fixed

- Fixed Windows releases corrupted archives.

### Removed

- [**BREAKING**] Removed `-s|--sanitize` option. It is done by default now, except if the `-t|--mime-type html` is set.
- [**BREAKING**] Removed `native-tls` support, `rustls-tls` is now the only TLS provider available. Removed in consequence `native-tls`, `rustls-tls` and `rustls-native-certs` cargo features.

## [0.7.3] - 2023-05-01

### Fixed

- Fixed Windows releases (due to typo in the github action script).
- Fixed unit tests.

## [0.7.2] - 2023-05-01

### Added

- Added `create` and `delete` folder command ([#54]).
- Added generated completions and man pages to release ([#43]).
- Added new account config option `sync-folders-strategy` which allows to choose a folders synchronization strateg ([#59]).:
  
  - `sync-folders-strategy = "all"`: synchronize all existing folders for the current account
  - `sync-folders-strategy.include = ["folder1", "folder2", …]`: synchronize only the given folders for the current account
  - `sync-folders-strategy.exclude = ["folder1", "folder2", …]`: synchronizes all folders except the given ones for the current account

  Also added new `account sync` arguments that override the account config option:
  
  - `-A|--all-folders`: include all folders to the synchronization.
  - `-F|--include-folder`: include given folders to the synchronization. They can be repeated `-F folder1 folder2` or `-F folder1 -F folder2`.
  - `-x|--exclude-folder`: exclude given folders from the synchronization. They can be repeated `-x folder1 folder2` or `-x folder1 -F folder2`.

- Added cargo features `native-tls` (default), `rustls-tls` and `rustls-native-certs`.

### Changed

- Made global options truly global, which means they can be used everywhere (not only *before* commands but also *after* ([#60]).
- Replaced reply all `-a` argument with `-A` because it conflicted with the global option `-a|--account`.
- Replaced `himalaya-lib` by `pimalaya-email`.
- Renamed feature `vendored` to `native-tls-vendored`.
- Removed the `develop` branch, all the development is now done on the `master` branch.

### Fixed

- Fixed config deserialization issue with `email-hooks` and `email-reading-format`.
- Fixed flags case sensitivity.

## [0.7.1] - 2023-02-14

### Added

- Added command `folders expunge` that deletes all emails marked for deletion.
  
### Changed

- Changed the location of the [documentation](https://pimalaya.org/himalaya/).

### Fixed

- Fixed broken links in README.md.

### Removed

- Removed the `maildir-backend` cargo feature, it is now included by default.
- Removed issues section on GitHub, now issues need to be opened by sending an email at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

## [0.7.0] - 2023-02-08

### Added

- Added offline support with the `account sync` command to synchronize a backend to a local Maildir backend.
- Added the flag `--disable-cache` to not use the local Maildir backend.
- Added the email composer (from its own [repository](https://git.sr.ht/~soywod/mime-msg-builder)).
- Added Musl builds to releases.
- Added `himalaya man` command to generate man page.

### Changed

- Made commands `read`, `attachments`, `flags`, `copy`, `move`, `delete` accept multiple ids.
- Flipped arguments `ids` and `folder` for commands `copy` and `move` in order the folder not to be considered as an id.

### Fixed

- Fixed missing folder aliases.

### Removed

- Removed the `-a|--attachment` argument from `write`, `reply` and `forward` commands. Instead you can attach documents directly from the template using the syntax `<#part filename=/path/to/you/document.ext>`.
- Removed the `-e|--encrypt` flag from `write`, `reply` and `forward` commands. Instead you can encrypt and sign parts directly from the template using the syntax `<#part type=text/plain encrypt=command sign=command>Hello!<#/part>`.
- Removed the `-l|--log-level` option, use instead the `RUST_LOG` environment variable (see the [wiki](https://github.com/soywod/himalaya/wiki/Tips:debug-and-logs))

## [0.6.1] - 2022-10-12

### Added

- Added `-s|--sanitize` flag for the `read` command.
  
### Changed

- Changed the behaviour of the `-t|--mime-type` argument of the `read` command. It is less strict now: if no part is found for the given MIME type, it will fallback to the other one. For example, giving `-t html` will show in priority HTML parts, but if none of them are found it will show plain parts instead (and vice versa).
- Sanitization is not done by default when using the `read` command, the flag `-s|--sanitize` needs to be explicitly provided.

### Fixed

- Fixed empty text bodies when reading html part on plain text email.

## [0.6.0] - 2022-10-10

### Changed

- Separated the CLI from the lib module.

  The source code has been split into subrepositories:

  - The email logic has been extracted from the CLI and placed in a lib on [SourceHut](https://git.sr.ht/~soywod/himalaya-lib)	
  - The vim plugin is now in a dedicated repository on [SourceHut](https://git.sr.ht/~soywod/himalaya-vim) as well
  - This repository only contains the CLI source code (it was not possible to move it to SourceHut because of cross platform builds)

- [**BREAKING**] Renamed `-m|--mailbox` to `-f|--folder`

- [**BREAKING**] Refactored config system.

  The configuration has been rethought in order to be more intuitive and structured. Here are the breaking changes for the global config:

  - `name` becomes `display-name` and is not mandatory anymore
  - `signature-delimiter` becomes `signature-delim`
  - `default-page-size` has been moved to `folder-listing-page-size` and `email-listing-page-size`
  - `notify-cmd`, `notify-query` and `watch-cmds` have been removed from the global config (available in account config only)
  - `folder-aliases` has been added to the global config (previously known as `mailboxes` from the account config)
  - `email-reading-headers`, `email-reading-format`,
    `email-reading-decrypt-cmd`, `email-writing-encrypt-cmd` and
    `email-hooks` have been added
  
  The account config inherits the same breaking changes from the global config, plus:

  - `imap-*` requires `backend = "imap"`
  - `maildir-*` requires `backend = "maildir"`
  - `notmuch-*` requires `backend = "notmuch"`
  - `smtp-*` requires `sender = "smtp"`
  - `sendmail-*` requires `sender = "sendmail"`
  - `pgp-encrypt-cmd` becomes `email-writing-encrypt-cmd`
  - `pgp-decrypt-cmd` becomes `email-reading-decrypt-cmd`
  - `mailboxes` becomes `folder-aliases`
  - `hooks` becomes `email-hooks`
  - `maildir-dir` becomes `maildir-root-dir`
  - `notmuch-database-dir` becomes `notmuch-db-path`

## [0.5.10] - 2022-03-20

### Fixed

- Fixed flag commands.
- Fixed Windows build.

## [0.5.9] - 2022-03-12

### Added

- SMTP pre-send hook
- Customize headers to show at the top of a read message

### Changed

- Improve `attachments` command

### Fixed

- `In-Reply-To` not set properly when replying to a message
- `Cc` missing or invalid when replying to a message
- Notmuch backend hangs
- Maildir e2e tests
- JSON API for listings

## [0.5.8] - 2022-03-04

### Added

- Flowed format support
- List accounts command
- One cargo feature per backend

### Changed

- Vim doc about mailbox pickers

### Fixed

- Some emojis break the table layout
- Bad sender and date in reply and forward template

## [0.5.7] - 2022-03-01

### Added

- Notmuch support

### Fixed

- Build failure due to `imap` version
- No tilde expansion in `maildir-dir`
- Unknown command SORT

### Changed

- [**BREAKING**] Replace `inbox-folder`, `sent-folder` and `draft-folder` by a generic hashmap `mailboxes`
- Display short envelopes id for `maildir` and `notmuch` backends

## [0.5.6] - 2022-02-22

### Added

- Sort command
- Maildir support

### Fixed

- Suffix to downloaded attachments with same name

## [0.5.5] - 2022-02-08

### Added

- [Contributing guide](https://github.com/soywod/himalaya/blob/master/CONTRIBUTING.md)
- Notify query config option
- End-to-end encryption

### Fixed

- Multiple recipients issue
- Cannot parse address

## [0.5.4] - 2022-02-05

### Fixed

- Add attachments with save and send commands
- Invalid sequence set

## [0.5.3] - 2022-02-03

### Added

- Activate rust-imap logs when trace mode is enabled
- Set up cargo deployment

## [0.5.2] - 2022-02-02

### Fixed

- Blur in list msg screenshot
- Make inbox, sent and drafts folders customizable
- Vim plugin get focused msg id
- Nix run issue
- Range not displayed when fetch fails
- Blank lines and spaces in `text/plain` parts
- Watch command
- Mailbox telescope.nvim preview

### Removed

- The wiki git submodule

## [0.5.1] - 2021-10-24

### Added

- Disable color feature
- `--max-width|-w` argument to restrict listing table width

### Fixed

- Error when receiving notification from `notify` command

### Changed

- Remove error when empty subject
- Vim plugin does not render anymore the msg by itself, it uses the one available from the CLI

## [0.5.0] - 2021-10-10

### Added

- Mailto support
- Remove previous signature when replying/forwarding a message
- Config option `signature-delimiter` to customize the signature delimiter (default to `-- \n`)
- Expand tilde and env vars for `downloads-dir` and `signature`

### Changed

- [**BREAKING**] Folder structure, message management, JSON API and Vim plugin
- Pagination for list and search cmd starts from 1 instead of 0
- Errors management with `anyhow`

### Fixed

- Panic on flags command
- Make more use of serde
- Write message vim plugin
- Invalid encoding when sending message
- Pagination reset current account
- New/reply/forward from Vim plugin since Tpl refactor

## [0.4.0] - 2021-06-03

### Added

- Add ability to change account in with the Vim plugin
- Add possibility to make Himalaya default email app

### Changed

- [**BREAKING**] Short version of reply `--all` arg is now `-A` to
  avoid conflicts with `--attachment|-a`
- Template management

### Fixed

- `\Seen` flag when moving a message
- Attachments arg for reply and forward commands
- Vim doc

### Removed

- `Content-Type` from templates

## [0.3.2] - 2021-05-08

### Added

- Mailbox attributes
- Wiki entry about new messages counter
- Copy/move/delete a message in vim

### Changed

- Get signature from file
- [**BREAKING**] Split `idle` command into two commands:
  - `notify`: Runs `notify-cmd` when a new message arrives to the server
  - `watch`: Runs `watch-cmds` when any change occurs on the server

### Removed

- `.exe` extension from release binaries

## [0.3.1] - 2021-05-04

### Added

- Send message via stdin

### Fixed

- Table with subject containing `\r`, `\n` or `\t`
- Overflow panic when shrink column
- Vim plugin empty mailbox message

## [0.3.0] - 2021-04-28

### Fixed

- IDLE mode after network interruption
- Output redirected to `stderr`
- Refactor table system
- Editon file format on Linux
- Show email address when name not available

### Removed

- `--log-level|-l` arg (replaced by default `RUST_LOG` env var from `env_logger`)

## [0.2.7] - 2021-04-24

### Added

- Default page size to config
- Custom config path
- Setting idle-hook-cmds

### Changed

- Plain logger with `env_logger`
- Refresh email list on load buffer

### Fixed

- Improve config compatibility on Windows
- Vim table containing emoji

## [0.2.6] - 2021-04-17

### Added

- Insecure TLS option
- Completion subcommands
- Vim flags to enable telescope preview and to choose picker

### Changed

- Make `install.sh` POSIX compliant

### Fixed

- SMTP port
- Save msg upon error
- Answered flag not set
- Panic when downloads-dir does not exist
- Idle mode incorrect new message notification

## [0.2.5] - 2021-04-12

### Fixed

- Expunge mbox after `move` and `delete` cmd
- JSON output

## [0.2.4] - 2021-04-09

### Added

- Wiki entry for Gmail users
- Info logs for copy/move/delete cmd + silent mode
- `--raw` arg for `read` cmd

### Changed

- Refactor output system + log levels

## [0.2.3] - 2021-04-08

### Added

- Telescope support

### Fixed

- Unicode chars breaks the view
- Copy/move incomplete (missing parts)

## [0.2.2] - 2021-04-04

### Added

- `w` alias for `write` cmd

### Fixed

- `attachments` cmd logs
- Page size arg `search` cmd

## [0.2.1] - 2021-04-04

### Added

- IDLE support
- Improve choice after editing msg
- Flags management
- Copy feature
- Move feature
- Delete feature
- Signature support
- Add attachment(s) to a message (CLI)

### Changed

- Errors management with `error_chain`

### Fixed

- Missing `FLAGS` column in messages table
- Subtract with overflow if next page empty

## [0.2.0] - 2021-03-10

### Added

- STARTTLS support
- Flags

### Changed

- JSON support

## [0.1.0] - 2021-01-17

### Added

- Parse TOML config
- Populate Config struct from TOML
- Set up IMAP connection
- List new emails
- Set up CLI arg parser
- List mailboxes command
- Text and HTML previews
- Set up SMTP connection
- Write new email
- Write new email
- Reply, reply all and forward
- Download attachments
- Merge `Email` with `Msg`
- List command with pagination
- Icon in table when attachment is present
- Multi-account
- Password from command
- Set up README

[#264]: https://github.com/pimalaya/himalaya/issues/264
[#469]: https://github.com/pimalaya/himalaya/issues/469
[#486]: https://github.com/pimalaya/himalaya/issues/486
[#492]: https://github.com/pimalaya/himalaya/issues/492
[#496]: https://github.com/pimalaya/himalaya/issues/496
[#508]: https://github.com/pimalaya/himalaya/issues/508
[#515]: https://github.com/pimalaya/himalaya/issues/515
[#518]: https://github.com/pimalaya/himalaya/issues/518
[#522]: https://github.com/pimalaya/himalaya/issues/522
[#523]: https://github.com/pimalaya/himalaya/issues/523
[#525]: https://github.com/pimalaya/himalaya/issues/525
[#535]: https://github.com/pimalaya/himalaya/issues/535
[#536]: https://github.com/pimalaya/himalaya/issues/536
[#538]: https://github.com/pimalaya/himalaya/issues/538
[#545]: https://github.com/pimalaya/himalaya/issues/545
[#547]: https://github.com/pimalaya/himalaya/issues/547
[#552]: https://github.com/pimalaya/himalaya/issues/552
[#559]: https://github.com/pimalaya/himalaya/issues/559
[#593]: https://github.com/pimalaya/himalaya/issues/593
[#611]: https://github.com/pimalaya/himalaya/issues/611
[#612]: https://github.com/pimalaya/himalaya/issues/612
[#627]: https://github.com/pimalaya/himalaya/issues/627
[#632]: https://github.com/pimalaya/himalaya/issues/632
[#634]: https://github.com/pimalaya/himalaya/issues/634

[core#1]: https://github.com/pimalaya/core/issues/1
[core#10]: https://github.com/pimalaya/core/issues/10

[unreleased]: https://github.com/pimalaya/himalaya/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/pimalaya/himalaya/compare/v1.2.0...v2.0.0
[1.2.0]: https://github.com/pimalaya/himalaya/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/pimalaya/himalaya/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/pimalaya/himalaya/compare/v1.0.0-beta.4...v1.0.0
[1.0.0-beta.4]: https://github.com/pimalaya/himalaya/compare/v1.0.0-beta.3...v1.0.0-beta.4
[1.0.0-beta.3]: https://github.com/pimalaya/himalaya/compare/v1.0.0-beta.2...v1.0.0-beta.3
[1.0.0-beta.2]: https://github.com/pimalaya/himalaya/compare/v1.0.0-beta...v1.0.0-beta.2
[1.0.0-beta]: https://github.com/pimalaya/himalaya/compare/v0.9.0...v1.0.0-beta
[0.9.0]: https://github.com/pimalaya/himalaya/compare/v0.8.4...v0.9.0
[0.8.4]: https://github.com/pimalaya/himalaya/compare/v0.8.3...v0.8.4
[0.8.3]: https://github.com/pimalaya/himalaya/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/pimalaya/himalaya/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/pimalaya/himalaya/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/pimalaya/himalaya/compare/v0.7.3...v0.8.0
[0.7.3]: https://github.com/pimalaya/himalaya/compare/v0.7.2...v0.7.3
[0.7.2]: https://github.com/pimalaya/himalaya/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/pimalaya/himalaya/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/pimalaya/himalaya/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/pimalaya/himalaya/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/pimalaya/himalaya/compare/v0.5.10...v0.6.0
[0.5.10]: https://github.com/pimalaya/himalaya/compare/v0.5.9...v0.5.10
[0.5.9]: https://github.com/pimalaya/himalaya/compare/v0.5.8...v0.5.9
[0.5.8]: https://github.com/pimalaya/himalaya/compare/v0.5.7...v0.5.8
[0.5.7]: https://github.com/pimalaya/himalaya/compare/v0.5.6...v0.5.7
[0.5.6]: https://github.com/pimalaya/himalaya/compare/v0.5.5...v0.5.6
[0.5.5]: https://github.com/pimalaya/himalaya/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/pimalaya/himalaya/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/pimalaya/himalaya/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/pimalaya/himalaya/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/pimalaya/himalaya/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/pimalaya/himalaya/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/pimalaya/himalaya/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/pimalaya/himalaya/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/pimalaya/himalaya/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/pimalaya/himalaya/compare/v0.2.7...v0.3.0
[0.2.7]: https://github.com/pimalaya/himalaya/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/pimalaya/himalaya/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/pimalaya/himalaya/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/pimalaya/himalaya/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/pimalaya/himalaya/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/pimalaya/himalaya/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/pimalaya/himalaya/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/pimalaya/himalaya/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/pimalaya/himalaya/releases/tag/v0.1.0
