# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0-beta] - 2023-12-31

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
- Disabled temporarily the `search` and `sort` command because they need to be refactored, see [#39](https://todo.sr.ht/~soywod/pimalaya/39).
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

- Fixed the way folder aliases are resolved. In some case, aliases were resolved CLI side and lib side, which led to alias errors [sourcehut#95].

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

- Added `create` and `delete` folder commands [sourcehut#54].
- Added generated completions and man pages to releases [sourcehut#43].
- Added new account config option `sync-folders-strategy` which allows to choose a folders synchronization strategy [sourcehut#59]:
  
  - `sync-folders-strategy = "all"`: synchronize all existing folders for the current account
  - `sync-folders-strategy.include = ["folder1", "folder2", …]`: synchronize only the given folders for the current account
  - `sync-folders-strategy.exclude = ["folder1", "folder2", …]`: synchronizes all folders except the given ones for the current account

  Also added new `account sync` arguments that override the account config option:
  
  - `-A|--all-folders`: include all folders to the synchronization.
  - `-F|--include-folder`: include given folders to the synchronization. They can be repeated `-F folder1 folder2` or `-F folder1 -F folder2`.
  - `-x|--exclude-folder`: exclude given folders from the synchronization. They can be repeated `-x folder1 folder2` or `-x folder1 -F folder2`.

- Added cargo features `native-tls` (default), `rustls-tls` and `rustls-native-certs`.

### Changed

- Made global options truly global, which means they can be used everywhere (not only *before* commands but also *after*) [sourcehut#60].
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

- Added offline support with the `account sync` command to synchronize a backend to a local Maildir backend [#342].
- Added the flag `--disable-cache` to not use the local Maildir backend.
- Added the email composer (from its own [repository](https://git.sr.ht/~soywod/mime-msg-builder)) [#341].
- Added Musl builds to releases [#356].
- Added `himalaya man` command to generate man page [#419].

### Changed

- Made commands `read`, `attachments`, `flags`, `copy`, `move`, `delete` accept multiple ids.
- Flipped arguments `ids` and `folder` for commands `copy` and `move` in order the folder not to be considered as an id.

### Fixed

- Fixed missing folder aliases [#430].

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

- Fixed empty text bodies when reading html part on plain text email [#352].

## [0.6.0] - 2022-10-10

### Changed

- Separated the CLI from the lib module [#340].

  The source code has been split into subrepositories:

  - The email logic has been extracted from the CLI and placed in a lib on [sourcehut](https://git.sr.ht/~soywod/himalaya-lib)	
  - The vim plugin is now in a dedicated repository on [sourcehut](https://git.sr.ht/~soywod/himalaya-vim) as well
  - This repository only contains the CLI source code (it was not possible to move it to sourcehut because of cross platform builds)

- [**BREAKING**] Renamed `-m|--mailbox` to `-f|--folder`

- [**BREAKING**] Refactored config system [#344].

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

- Fixed flag commands [#334].
- Fixed Windows build [#346].

## [0.5.9] - 2022-03-12

### Added

- SMTP pre-send hook [#178]
- Customize headers to show at the top of a read message [#338]

### Changed

- Improve `attachments` command [#281]

### Fixed

- `In-Reply-To` not set properly when replying to a message [#323]
- `Cc` missing or invalid when replying to a message [#324]
- Notmuch backend hangs [#329]
- Maildir e2e tests [#335]
- JSON API for listings [#331]

## [0.5.8] - 2022-03-04

### Added

- Flowed format support [#206]
- List accounts command [#244]
- One cargo feature per backend [#318]

### Changed

- Vim doc about mailbox pickers [#298]

### Fixed

- Some emojis break the table layout [#300]
- Bad sender and date in reply and forward template [#321]

## [0.5.7] - 2022-03-01

### Added

- Notmuch support [#57]

### Fixed

- Build failure due to `imap` version [#303]
- No tilde expansion in `maildir-dir` [#305]
- Unknown command SORT [#308]

### Changed

- [**BREAKING**] Replace `inbox-folder`, `sent-folder` and `draft-folder` by a generic hashmap `mailboxes`
- Display short envelopes id for `maildir` and `notmuch` backends [#309]

## [0.5.6] - 2022-02-22

### Added

- Sort command [#34]
- Maildir support [#43]

### Fixed

- Suffix to downloaded attachments with same name [#204]

## [0.5.5] - 2022-02-08

### Added

- [Contributing guide](https://github.com/soywod/himalaya/blob/master/CONTRIBUTING.md) [#256]
- Notify query config option [#289]
- End-to-end encryption [#54]

### Fixed

- Multiple recipients issue [#288]
- Cannot parse address [#227]

## [0.5.4] - 2022-02-05

### Fixed

- Add attachments with save and send commands [#47] [#259]
- Invalid sequence set [#276]

## [0.5.3] - 2022-02-03

### Added

- Activate rust-imap logs when trace mode is enabled
- Set up cargo deployment

## [0.5.2] - 2022-02-02

### Fixed

- Blur in list msg screenshot [#181]
- Make inbox, sent and drafts folders customizable [#172]
- Vim plugin get focused msg id [#268]
- Nix run issue [#272]
- Range not displayed when fetch fails [#276]
- Blank lines and spaces in `text/plain` parts [#280]
- Watch command [#271]
- Mailbox telescope.nvim preview [#249]

### Removed

- The wiki git submodule [#273]

## [0.5.1] - 2021-10-24

### Added

- Disable color feature [#185]
- `--max-width|-w` argument to restrict listing table width [#220]

### Fixed

- Error when receiving notification from `notify` command [#228]

### Changed

- Remove error when empty subject [#229]
- Vim plugin does not render anymore the msg by itself, it uses the one available from the CLI [#220]

## [0.5.0] - 2021-10-10

### Added

- Mailto support [#162]
- Remove previous signature when replying/forwarding a message [#193]
- Config option `signature-delimiter` to customize the signature delimiter (default to `-- \n`) [[#114](https://github.com/soywod/himalaya/pull/114)]
- Expand tilde and env vars for `downloads-dir` and `signature` [#102]

### Changed

- [**BREAKING**] Folder structure, message management, JSON API and Vim plugin [#199]
- Pagination for list and search cmd starts from 1 instead of 0 [#186]
- Errors management with `anyhow` [#152]

### Fixed

- Panic on flags command [#190]
- Make more use of serde [#153]
- Write message vim plugin [#196]
- Invalid encoding when sending message [#205]
- Pagination reset current account [#215]
- New/reply/forward from Vim plugin since Tpl refactor [#176]

## [0.4.0] - 2021-06-03

### Added

- Add ability to change account in with the Vim plugin [#91]
- Add possibility to make Himalaya default email app [#160] [[#161](https://github.com/soywod/himalaya/pull/161)]

### Changed

- [**BREAKING**] Short version of reply `--all` arg is now `-A` to
  avoid conflicts with `--attachment|-a`
- Template management [#80]

### Fixed

- `\Seen` flag when moving a message
- Attachments arg for reply and forward commands [#109]
- Vim doc [#117]

### Removed

- `Content-Type` from templates [#146]

## [0.3.2] - 2021-05-08

### Added

- Mailbox attributes [#134]
- Wiki entry about new messages counter [#121]
- Copy/move/delete a message in vim [#95]

### Changed

- Get signature from file [#135]
- [**BREAKING**] Split `idle` command into two commands:
  - `notify`: Runs `notify-cmd` when a new message arrives to the server
  - `watch`: Runs `watch-cmds` when any change occurs on the server

### Removed

- `.exe` extension from release binaries [#144]

## [0.3.1] - 2021-05-04

### Added

- Send message via stdin [#78]

### Fixed

- Table with subject containing `\r`, `\n` or `\t` [#141]
- Overflow panic when shrink column [#138]
- Vim plugin empty mailbox message [#136]

## [0.3.0] - 2021-04-28

### Fixed

- IDLE mode after network interruption [#123]
- Output redirected to `stderr` [#130]
- Refactor table system [#132]
- Editon file format on Linux [#133]
- Show email address when name not available [#131]

### Removed

- `--log-level|-l` arg (replaced by default `RUST_LOG` env var from `env_logger`) [#130]

## [0.2.7] - 2021-04-24

### Added

- Default page size to config [#96]
- Custom config path [#86]
- Setting idle-hook-cmds

### Changed

- Plain logger with `env_logger` [#126]
- Refresh email list on load buffer [#125]

### Fixed

- Improve config compatibility on Windows [[#111](https://github.com/soywod/himalaya/pull/111)]
- Vim table containing emoji [#122]

## [0.2.6] - 2021-04-17

### Added

- Insecure TLS option [#84] [#103](https://github.com/soywod/himalaya/pull/103) [[#105](https://github.com/soywod/himalaya/pull/105)]
- Completion subcommands [[#99](https://github.com/soywod/himalaya/pull/99)]
- Vim flags to enable telescope preview and to choose picker [[#97](https://github.com/soywod/himalaya/pull/97)]

### Changed

- Make `install.sh` POSIX compliant [[#53](https://github.com/soywod/himalaya/pull/53)]

### Fixed

- SMTP port [#87]
- Save msg upon error [#59]
- Answered flag not set [#50]
- Panic when downloads-dir does not exist [#100]
- Idle mode incorrect new message notification [#48]

## [0.2.5] - 2021-04-12

### Fixed

- Expunge mbox after `move` and `delete` cmd [#83]
- JSON output [#89]

## [0.2.4] - 2021-04-09

### Added

- Wiki entry for Gmail users [#58]
- Info logs for copy/move/delete cmd + silent mode [#74]
- `--raw` arg for `read` cmd [#79]

### Changed

- Refactor output system + log levels [#74]

## [0.2.3] - 2021-04-08

### Added

- Telescope support [#61]

### Fixed

- Unicode chars breaks the view [#71]
- Copy/move incomplete (missing parts) [#75]

## [0.2.2] - 2021-04-04

### Added

- `w` alias for `write` cmd

### Fixed

- `attachments` cmd logs
- Page size arg `search` cmd

## [0.2.1] - 2021-04-04

### Added

- IDLE support [#29]
- Improve choice after editing msg [#30]
- Flags management [#41]
- Copy feature [#35]
- Move feature [#31]
- Delete feature [#36]
- Signature support [#33]
- Add attachment(s) to a message (CLI) [#37]

### Changed

- Errors management with `error_chain` [#39]

### Fixed

- Missing `FLAGS` column in messages table [#40]
- Subtract with overflow if next page empty [#38]

## [0.2.0] - 2021-03-10

### Added

- STARTTLS support [#32]
- Flags [#25]

### Changed

- JSON support [#18]

## [0.1.0] - 2021-01-17

### Added

- Parse TOML config [#1]
- Populate Config struct from TOML [#2]
- Set up IMAP connection [#3]
- List new emails [#6]
- Set up CLI arg parser [#15]
- List mailboxes command [#5]
- Text and HTML previews [#12] [#13]
- Set up SMTP connection [#4]
- Write new email [#8]
- Write new email [#8]
- Reply, reply all and forward [#9] [#10] [#11]
- Download attachments [#14]
- Merge `Email` with `Msg` [#21]
- List command with pagination [#19]
- Icon in table when attachment is present [#16]
- Multi-account [#17]
- Password from command [#22]
- Set up README [#20]

[Unreleased]: https://github.com/soywod/himalaya/compare/v1.0.0-beta...HEAD
[1.0.0-beta]: https://github.com/soywod/himalaya/compare/v0.9.0...v1.0.0-beta
[0.9.0]: https://github.com/soywod/himalaya/compare/v0.8.4...v0.9.0
[0.8.4]: https://github.com/soywod/himalaya/compare/v0.8.3...v0.8.4
[0.8.3]: https://github.com/soywod/himalaya/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/soywod/himalaya/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/soywod/himalaya/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/soywod/himalaya/compare/v0.7.3...v0.8.0
[0.7.3]: https://github.com/soywod/himalaya/compare/v0.7.2...v0.7.3
[0.7.2]: https://github.com/soywod/himalaya/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/soywod/himalaya/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/soywod/himalaya/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/soywod/himalaya/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/soywod/himalaya/compare/v0.5.10...v0.6.0
[0.5.10]: https://github.com/soywod/himalaya/compare/v0.5.9...v0.5.10
[0.5.9]: https://github.com/soywod/himalaya/compare/v0.5.8...v0.5.9
[0.5.8]: https://github.com/soywod/himalaya/compare/v0.5.7...v0.5.8
[0.5.7]: https://github.com/soywod/himalaya/compare/v0.5.6...v0.5.7
[0.5.6]: https://github.com/soywod/himalaya/compare/v0.5.5...v0.5.6
[0.5.5]: https://github.com/soywod/himalaya/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/soywod/himalaya/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/soywod/himalaya/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/soywod/himalaya/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/soywod/himalaya/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/soywod/himalaya/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/soywod/himalaya/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/soywod/himalaya/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/soywod/himalaya/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/soywod/himalaya/compare/v0.2.7...v0.3.0
[0.2.7]: https://github.com/soywod/himalaya/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/soywod/himalaya/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/soywod/himalaya/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/soywod/himalaya/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/soywod/himalaya/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/soywod/himalaya/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/soywod/himalaya/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/soywod/himalaya/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/soywod/himalaya/releases/tag/v0.1.0

[#1]: https://github.com/soywod/himalaya/issues/1
[#2]: https://github.com/soywod/himalaya/issues/2
[#3]: https://github.com/soywod/himalaya/issues/3
[#4]: https://github.com/soywod/himalaya/issues/4
[#5]: https://github.com/soywod/himalaya/issues/5
[#8]: https://github.com/soywod/himalaya/issues/8
[#9]: https://github.com/soywod/himalaya/issues/9
[#10]: https://github.com/soywod/himalaya/issues/10
[#11]: https://github.com/soywod/himalaya/issues/11
[#12]: https://github.com/soywod/himalaya/issues/12
[#13]: https://github.com/soywod/himalaya/issues/13
[#14]: https://github.com/soywod/himalaya/issues/14
[#15]: https://github.com/soywod/himalaya/issues/15
[#16]: https://github.com/soywod/himalaya/issues/16
[#17]: https://github.com/soywod/himalaya/issues/17
[#18]: https://github.com/soywod/himalaya/issues/18
[#19]: https://github.com/soywod/himalaya/issues/19
[#20]: https://github.com/soywod/himalaya/issues/20
[#21]: https://github.com/soywod/himalaya/issues/21
[#22]: https://github.com/soywod/himalaya/issues/22
[#25]: https://github.com/soywod/himalaya/issues/25
[#29]: https://github.com/soywod/himalaya/issues/29
[#30]: https://github.com/soywod/himalaya/issues/30
[#31]: https://github.com/soywod/himalaya/issues/31
[#32]: https://github.com/soywod/himalaya/issues/32
[#33]: https://github.com/soywod/himalaya/issues/33
[#34]: https://github.com/soywod/himalaya/issues/34
[#35]: https://github.com/soywod/himalaya/issues/35
[#37]: https://github.com/soywod/himalaya/issues/37
[#38]: https://github.com/soywod/himalaya/issues/38
[#39]: https://github.com/soywod/himalaya/issues/39
[#40]: https://github.com/soywod/himalaya/issues/40
[#41]: https://github.com/soywod/himalaya/issues/41
[#43]: https://github.com/soywod/himalaya/issues/43
[#47]: https://github.com/soywod/himalaya/issues/47
[#48]: https://github.com/soywod/himalaya/issues/48
[#50]: https://github.com/soywod/himalaya/issues/50
[#54]: https://github.com/soywod/himalaya/issues/54
[#57]: https://github.com/soywod/himalaya/issues/57
[#58]: https://github.com/soywod/himalaya/issues/58
[#59]: https://github.com/soywod/himalaya/issues/59
[#61]: https://github.com/soywod/himalaya/issues/61
[#71]: https://github.com/soywod/himalaya/issues/71
[#74]: https://github.com/soywod/himalaya/issues/74
[#75]: https://github.com/soywod/himalaya/issues/75
[#78]: https://github.com/soywod/himalaya/issues/78
[#79]: https://github.com/soywod/himalaya/issues/79
[#80]: https://github.com/soywod/himalaya/issues/80
[#83]: https://github.com/soywod/himalaya/issues/83
[#84]: https://github.com/soywod/himalaya/issues/84
[#86]: https://github.com/soywod/himalaya/issues/86
[#87]: https://github.com/soywod/himalaya/issues/87
[#89]: https://github.com/soywod/himalaya/issues/89
[#91]: https://github.com/soywod/himalaya/issues/91
[#95]: https://github.com/soywod/himalaya/issues/95
[#96]: https://github.com/soywod/himalaya/issues/96
[#100]: https://github.com/soywod/himalaya/issues/100
[#102]: https://github.com/soywod/himalaya/issues/102
[#109]: https://github.com/soywod/himalaya/issues/109
[#117]: https://github.com/soywod/himalaya/issues/117
[#121]: https://github.com/soywod/himalaya/issues/121
[#122]: https://github.com/soywod/himalaya/issues/122
[#123]: https://github.com/soywod/himalaya/issues/123
[#125]: https://github.com/soywod/himalaya/issues/125
[#126]: https://github.com/soywod/himalaya/issues/126
[#130]: https://github.com/soywod/himalaya/issues/130
[#131]: https://github.com/soywod/himalaya/issues/131
[#132]: https://github.com/soywod/himalaya/issues/132
[#133]: https://github.com/soywod/himalaya/issues/133
[#134]: https://github.com/soywod/himalaya/issues/134
[#135]: https://github.com/soywod/himalaya/issues/135
[#136]: https://github.com/soywod/himalaya/issues/136
[#138]: https://github.com/soywod/himalaya/issues/138
[#141]: https://github.com/soywod/himalaya/issues/141
[#144]: https://github.com/soywod/himalaya/issues/144
[#146]: https://github.com/soywod/himalaya/issues/146
[#152]: https://github.com/soywod/himalaya/issues/152
[#153]: https://github.com/soywod/himalaya/issues/153
[#160]: https://github.com/soywod/himalaya/issues/160
[#162]: https://github.com/soywod/himalaya/issues/162
[#176]: https://github.com/soywod/himalaya/issues/176
[#172]: https://github.com/soywod/himalaya/issues/172
[#178]: https://github.com/soywod/himalaya/issues/178
[#181]: https://github.com/soywod/himalaya/issues/181
[#185]: https://github.com/soywod/himalaya/issues/185
[#186]: https://github.com/soywod/himalaya/issues/186
[#190]: https://github.com/soywod/himalaya/issues/190
[#193]: https://github.com/soywod/himalaya/issues/193
[#196]: https://github.com/soywod/himalaya/issues/196
[#199]: https://github.com/soywod/himalaya/issues/199
[#204]: https://github.com/soywod/himalaya/issues/204
[#205]: https://github.com/soywod/himalaya/issues/205
[#206]: https://github.com/soywod/himalaya/issues/206
[#215]: https://github.com/soywod/himalaya/issues/215
[#220]: https://github.com/soywod/himalaya/issues/220
[#227]: https://github.com/soywod/himalaya/issues/227
[#228]: https://github.com/soywod/himalaya/issues/228
[#229]: https://github.com/soywod/himalaya/issues/229
[#244]: https://github.com/soywod/himalaya/issues/244
[#249]: https://github.com/soywod/himalaya/issues/249
[#256]: https://github.com/soywod/himalaya/issues/256
[#259]: https://github.com/soywod/himalaya/issues/259
[#268]: https://github.com/soywod/himalaya/issues/268
[#272]: https://github.com/soywod/himalaya/issues/272
[#273]: https://github.com/soywod/himalaya/issues/273
[#276]: https://github.com/soywod/himalaya/issues/276
[#271]: https://github.com/soywod/himalaya/issues/271
[#276]: https://github.com/soywod/himalaya/issues/276
[#280]: https://github.com/soywod/himalaya/issues/280
[#281]: https://github.com/soywod/himalaya/issues/281
[#288]: https://github.com/soywod/himalaya/issues/288
[#289]: https://github.com/soywod/himalaya/issues/289
[#298]: https://github.com/soywod/himalaya/issues/298
[#300]: https://github.com/soywod/himalaya/issues/300
[#303]: https://github.com/soywod/himalaya/issues/303
[#305]: https://github.com/soywod/himalaya/issues/305
[#308]: https://github.com/soywod/himalaya/issues/308
[#309]: https://github.com/soywod/himalaya/issues/309
[#318]: https://github.com/soywod/himalaya/issues/318
[#321]: https://github.com/soywod/himalaya/issues/321
[#323]: https://github.com/soywod/himalaya/issues/323
[#324]: https://github.com/soywod/himalaya/issues/324
[#329]: https://github.com/soywod/himalaya/issues/329
[#331]: https://github.com/soywod/himalaya/issues/331
[#334]: https://github.com/soywod/himalaya/issues/334
[#335]: https://github.com/soywod/himalaya/issues/335
[#338]: https://github.com/soywod/himalaya/issues/338
[#340]: https://github.com/soywod/himalaya/issues/340
[#341]: https://github.com/soywod/himalaya/issues/341
[#342]: https://github.com/soywod/himalaya/issues/342
[#344]: https://github.com/soywod/himalaya/issues/344
[#346]: https://github.com/soywod/himalaya/issues/346
[#352]: https://github.com/soywod/himalaya/issues/352
[#356]: https://github.com/soywod/himalaya/issues/356
[#419]: https://github.com/soywod/himalaya/issues/419
[#430]: https://github.com/soywod/himalaya/issues/430

[sourcehut#43]: https://todo.sr.ht/~soywod/pimalaya/43
[sourcehut#54]: https://todo.sr.ht/~soywod/pimalaya/54
[sourcehut#59]: https://todo.sr.ht/~soywod/pimalaya/59
[sourcehut#60]: https://todo.sr.ht/~soywod/pimalaya/60
[sourcehut#95]: https://todo.sr.ht/~soywod/pimalaya/95
