# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Disable color support [#185]

### Fixed

- Error when receiving notification from `notify` command [#228]

### Change

- Remove error when empty subject [#229]

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

- [**BREAKING**] Short version of reply `--all` arg is now `-A` to avoid conflicts with `--attachment|-a`
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

[unreleased]: https://github.com/soywod/himalaya/compare/v0.5.0...HEAD
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
[#48]: https://github.com/soywod/himalaya/issues/48
[#50]: https://github.com/soywod/himalaya/issues/50
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
[#185]: https://github.com/soywod/himalaya/issues/185
[#186]: https://github.com/soywod/himalaya/issues/186
[#190]: https://github.com/soywod/himalaya/issues/190
[#193]: https://github.com/soywod/himalaya/issues/193
[#196]: https://github.com/soywod/himalaya/issues/196
[#199]: https://github.com/soywod/himalaya/issues/199
[#205]: https://github.com/soywod/himalaya/issues/205
[#215]: https://github.com/soywod/himalaya/issues/215
[#228]: https://github.com/soywod/himalaya/issues/228
[#229]: https://github.com/soywod/himalaya/issues/229
