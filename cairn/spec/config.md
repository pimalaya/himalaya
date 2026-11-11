---
cairn: spec
capability: config
status: current
---

# Config

Himalaya is configured through TOML: a top-level block plus named account blocks, one table per account under `[accounts.<name>]`, each carrying optional per-backend sub-blocks. The config schema is a set of pure DTOs (`*Config` types) mirroring the nested TOML shape; the selected account is flattened into a runtime `Account` view that commands consume. Config files stay user-owned: Himalaya never writes them.

### Requirement: Multi-account schema
The config SHALL be multi-account: a top-level block holding shared defaults, plus `[accounts.<name>]` blocks. Each account carries at most one storage backend sub-block (`imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `m2dir`, `mbox`, `pimdir`) and optional service sub-blocks for `smtp` and `sieve`.

The `sieve` block SHALL accept `sieve://`, `sieves://`, and `unix://` servers, and SHALL expose the shared TLS and SASL vocabulary, every mechanism the other backends accept reaching ManageSieve too.

A bare authority SHALL resolve to `sieve://`, where the `imap` and `smtp` blocks resolve theirs to the implicit-TLS scheme: RFC 5804 registers one port, 4190, and defines STARTTLS as the way to TLS on it, so an implicit-TLS default would reach nothing on a stock server. `sieves://` SHALL stay accepted for the deployments listening for a TLS handshake straight away. `sieve.starttls` left unset SHALL follow the scheme, on for `sieve://` and off for the other two, and setting it on a `sieves://` server SHALL be an error.

A mechanism disclosing a reusable credential SHALL be refused over a cleartext connection unless `sieve.allow-cleartext-auth` is set, which is the configuration RFC 5804 section 5 asks for.

### Requirement: Config loading and merge
Without `-c/--config`, the config SHALL load every existing canonical path as a layer, in this order: `$XDG_CONFIG_HOME/himalaya/config.toml`, `$HOME/.config/himalaya/config.toml`, `$HOME/.himalayarc`. Each layer SHALL be deep-merged on top of the ones before, so `$HOME/.himalayarc` overrides. Paths naming the same file SHALL be read once.

`-c/--config` SHALL replace the canonical paths entirely. It MAY be repeated: the first path is the base and the next ones are deep-merged on top, in order. Each occurrence SHALL be one path, never split on a delimiter, and no environment variable SHALL be read for it.

#### Scenario: A Windows path is read whole
- GIVEN `-c C:\Users\me\config.toml`
- WHEN the configuration is loaded
- THEN it is read from that one path, not from `C` and `\Users\me\config.toml`

### Requirement: Missing account is an error
When the config exists but lacks the requested account, the command SHALL fail naming it and listing the accounts the configuration holds, sorted. When no account is named and none is marked default, it SHALL fail naming both ways of picking one. When no configuration is found at all, a command needing one SHALL fail naming the path it looked at (the first `-c` path, else the default location) and pointing at the documented sample, unless a person meets the welcome instead. Himalaya SHALL NOT prompt to create a configuration.

### Requirement: A missing configuration meets a person with the welcome
When no configuration exists at the first `-c` path or the default location, and stderr is a terminal with `--json` off, a bare `himalaya`, every command resolving an account (`repl` and `mcp` included) and `account list` SHALL print the welcome on stderr and exit 1, printing nothing on stdout. The welcome SHALL frame Himalaya in one sentence, name the missing path, give the commands creating it from config.sample.toml, invite the reader to edit it and uncomment the options they need, and give the `envelope list` command carrying the `-c` paths. When share/doc/himalaya/config.sample.toml exists under the prefix holding the binary's directory or under that directory itself, symlinks resolved, the commands SHALL copy it there with a writable mode, after creating the missing directory; otherwise they SHALL download the raw config.sample.toml with `curl --create-dirs`. Every path SHALL be quoted for the platform's shells, so each command runs as printed. Otherwise the missing-configuration error SHALL stand, as the failure object under `--json`.

#### Scenario: A first run
- GIVEN no file at $XDG_CONFIG_HOME/himalaya/config.toml, $HOME/.config/himalaya/config.toml or $HOME/.himalayarc
- WHEN `himalaya envelope list` runs in a terminal
- THEN the welcome names $XDG_CONFIG_HOME/himalaya/config.toml, the `curl` command downloading config.sample.toml there and `himalaya envelope list`, and the exit status is 1

#### Scenario: A script
- GIVEN no configuration
- WHEN `himalaya --json envelope list` runs
- THEN stdout carries the failure object naming the path looked at and the sample, stderr carries no welcome, and the exit status is 1

### Requirement: A bare invocation shows the help
A bare `himalaya` SHALL print the help when a configuration exists, a broken one included, its error left to the first command reading it. Without one it SHALL fail as a command resolving an account does, the welcome or the error. Nothing prompts.

### Requirement: One default account
Resolving the default account SHALL fail, naming them, when several accounts are marked `default = true` once the configuration is loaded, layers and `-c` files merged. An account named with `-a` SHALL still resolve.

### Requirement: Account identity
An account MAY declare the address it sends as, `email`, and the name that address carries, `display-name`. `display-name` MAY also be declared at the top level, where it applies to every account that does not override it. Neither is required, and neither is validated as an addr-spec: an account that never composes has no use for them.

Both keys SHALL also be accepted under the spellings himalaya-tui writes, `from` and `from-name`, and himalaya-tui SHALL accept `email` and `display-name` in turn, the two binaries sharing one configuration file.

### Requirement: Account signature
An account MAY declare a `signature` and the `signature-delim` introducing it, and both MAY also be declared at the top level, where they apply to every account that does not override them.

`signature` SHALL be the signature alone. `signature-delim` SHALL default to the RFC 3676 §4.3 `"-- \n"` and SHALL be written verbatim, its own trailing newline included, so a delimiter meant to stand on its own line says so rather than relying on a rule. Both binaries SHALL assemble the block from the same two keys, so one configured value reads the same whichever composes.

### Requirement: Mailbox aliases
An account MAY map friendly mailbox names to backend-native ids under `[accounts.<name>.mailbox.alias]`. Alias names are case-insensitive on lookup and on storage. `-m/--mailbox` resolves an alias first, then a mailbox role, then passes the value verbatim; a role matching several mailboxes is an error. A shared command that omits `-m/--mailbox` resolves the `inbox` alias, else the `inbox` role. Account-level entries override same-named global entries, and ids are stored verbatim.

### Requirement: Config never written
Himalaya SHALL NOT write a configuration file: the user writes it, from the documented sample.

### Requirement: The mbox block
An `mbox` block SHALL name `root`, the directory of mbox files, and MAY name `inbox` (the spool), `format` (`mboxo`, `mboxrd` by default, `mboxcl`, `mboxcl2`, which also decides whether `Content-Length` is trusted on read), `thunderbird` (the `.sbd` layout, off by default) and `lock.dotlock`, `lock.fcntl` (both on by default) and `lock.timeout`.

### Requirement: Path keys expand as the configuration is read
Every path-valued key SHALL expand `~` and environment variables while the configuration is deserialized, not where it is read. This covers `maildir.root`, `m2dir.root`, `mbox.root`, `mbox.inbox`, `pimdir.root`, the global and per-account `downloads-dir`, and the `tls.cert` of every backend.

An expansion that fails, an undefined variable being the case, SHALL leave the raw value untouched rather than fail the load.

A reader SHALL therefore receive an already-expanded path and SHALL NOT expand again: expansion at a call site holds only where somebody remembered it, which is how `maildir.root = "~/Mail"` came to open a literal `./~/Mail`.

### Requirement: A credential command is spawned once per run
A command backing a secret SHALL be spawned once per command run, however many blocks of the account name it, and its value SHALL be handed to every field naming it. A run that reaches IMAP, SMTP and ManageSieve therefore unlocks a `pass` or `gpg` entry once rather than three times.

Two commands SHALL count as one only where the configuration wrote them identically: the shell-string form and the argv-array form are distinct even when they run the same program.

The resolved value SHALL live no longer than the run that resolved it, and SHALL never be written to a config file, a log line or a cache.

### Requirement: A proxy is set per account and per backend
An account's `proxy` SHALL apply to every network backend of that account (IMAP, SMTP, ManageSieve, JMAP, Gmail, Microsoft Graph) whose own block names no `proxy`, and a backend's own `proxy` SHALL win over it. With neither, the connection SHALL read the `all_proxy` and `https_proxy` environment variables, honouring `no_proxy`.

`proxy.url` SHALL take `socks5://`, `socks5h://` or `http://`. `proxy.username` and `proxy.password` SHALL override the URL's user info, the password being a secret like any credential; a password without a username SHALL be rejected.

### Requirement: A v1 account layout is named, not silently ignored
Unknown keys SHALL stay tolerated, the file being shared with himalaya-tui. When no configured backend matches `--backend`, the error SHALL name the account and, under `auto`, the backends the command supports in this build; under a named backend, the missing block, or that the backend is unsupported. An account carrying the v1 `backend` table, which no binary reads since v2, SHALL load as if the table were absent, and that error SHALL name the table and point at MIGRATION.md.

### Requirement: Envelope dates MAY render relative to today
`envelope.list.datetime-relative`, settable globally and per account, SHALL render the `Date` field of `envelope list` and `envelope search` relative to today in the local timezone: the time of day (`%R`) for today, `yesterday` for the day before, the weekday (`%A`) for the five days before that, and `datetime-fmt` for anything older or dated in the future. It SHALL default to `false`, and SHALL NOT change the `--json` output, which keeps the exact date.

### Requirement: Sent copies are configured per account
`message.send.save-copy`, global or per account, SHALL be the mailbox a sending command (`message send`, and `message compose`, `message reply` and `message forward` with `--send`) appends a copy to when `--save` is not passed. It takes a mailbox name, alias or role, resolved as `--save` is; `true` SHALL stand for `sent` and `false` for no copy. `--no-save` SHALL skip it for one call and conflict with `--save`. The `message` table SHALL accept unknown keys, so a v1 `[message]` table keeps loading.

### Requirement: Posting styles are configured per account
`message.reply.posting-style` and `message.forward.posting-style`, global or per account, SHALL take `top`, `bottom` or `none` and SHALL be the posting style `message reply` and `message forward` use when `--posting-style` is not passed. With neither, the style SHALL be `top`.

### Requirement: No colour is configured
The configuration SHALL hold no colour, no table shape and no glyph: `*.list.table.*-color`, `table.preset` and `table.arrangement` are unknown keys, refused where their table denies unknown keys (`mailbox`) and ignored at the top level and the account level, which tolerate unknown keys for himalaya-tui. The `envelope.list.table` block SHALL be ignored whatever it holds, its v2 glyph and colour keys included, while `envelope.list` keeps refusing an unknown key of its own.

#### Scenario: A v2 glyph key
- GIVEN an account setting `envelope.list.table.unseen-char = "N"`
- WHEN `envelope list` runs
- THEN the configuration loads and an unread message reads `unread` among its state words

#### Scenario: A typo under envelope.list
- GIVEN an account setting `envelope.list.datetime-fnt`
- WHEN any command reads the configuration
- THEN it is refused naming `datetime-fnt`

### Requirement: One sample configuration, with a section per backend
The repository SHALL ship config.sample.toml holding one `[accounts.example]` table with dotted keys and no other table header. It SHALL list the options every account shares and each of their values, the default active and the others commented as `#key = value`, every line ending in one tab-aligned comment, and every password command a failing `exit 127` placeholder. It SHALL end with one section per backend or provider (IMAP and SMTP, ManageSieve, JMAP, Gmail, Microsoft Graph, Microsoft IMAP and SMTP, Fastmail, Maildir, m2dir, mbox, pimdir), each a paragraph of commented code headed by its name, its own alternatives commented twice, an OAuth 2.0 token read through `ortie token show -a example`, so the file holds a working account once one section is uncommented. The test suite SHALL parse config.sample.toml through the loader `-c` uses with each section uncommented whole, then on top of it each line commented twice inside it and each line of commented code outside the sections, in place of the active lines of its field, and SHALL resolve `example` as the default account each time, so a configuration change breaking one fails the build.

#### Scenario: A field is renamed
- GIVEN config.sample.toml, whose IMAP and SMTP section sets `imap.sort.fallback`
- WHEN the field is renamed in the configuration types and the sample is not updated
- THEN the test suite fails naming the sample and the line uncommented
