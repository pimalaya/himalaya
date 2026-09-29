---
cairn: spec
capability: config
status: current
---

# Config

Himalaya is configured through TOML: a top-level block plus named account blocks, one table per account under `[accounts.<name>]`, each carrying optional per-backend sub-blocks. The config schema is a set of pure DTOs (`*Config` types) mirroring the nested TOML shape; the selected account is flattened into a runtime `Account` view that commands consume. Config files stay user-owned: Himalaya never writes them.

### Requirement: Multi-account schema
The config SHALL be multi-account: a top-level block holding shared defaults, plus `[accounts.<name>]` blocks. Each account carries at most one storage backend sub-block (`imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `m2dir`) and optional service sub-blocks for `smtp` and `sieve`.

The `sieve` block SHALL accept `sieve://`, `sieves://`, and `unix://` servers, and SHALL expose the shared TLS and SASL vocabulary, every mechanism the other backends accept reaching ManageSieve too.

A bare authority SHALL resolve to `sieve://`, where the `imap` and `smtp` blocks resolve theirs to the implicit-TLS scheme: RFC 5804 registers one port, 4190, and defines STARTTLS as the way to TLS on it, so an implicit-TLS default would reach nothing on a stock server. `sieves://` SHALL stay accepted for the deployments listening for a TLS handshake straight away. `sieve.starttls` left unset SHALL follow the scheme, on for `sieve://` and off for the other two, and setting it on a `sieves://` server SHALL be an error.

A mechanism disclosing a reusable credential SHALL be refused over a cleartext connection unless `sieve.allow-cleartext-auth` is set, which is the configuration RFC 5804 section 5 asks for.

### Requirement: Config loading and merge
The config SHALL load from the first existing canonical path (`$XDG_CONFIG_HOME/himalaya/config.toml`, `$HOME/.config/himalaya/config.toml`, `$HOME/.himalayarc`), overridable with `-c/--config`. Multiple `-c` paths MAY be passed, colon-free as repeated flags: the first is the base and the rest are deep-merged on top.

### Requirement: Missing account is an error
When the config exists but lacks the requested account, the command SHALL fail with a hard error, not fall back to the wizard. The wizard is proposed only when no config is found at all.

### Requirement: Account identity
An account MAY declare the address it sends as, `email`, and the name that address carries, `display-name`. `display-name` MAY also be declared at the top level, where it applies to every account that does not override it. Neither is required, and neither is validated as an addr-spec: an account that never composes has no use for them.

Both keys SHALL also be accepted under the spellings himalaya-tui writes, `from` and `from-name`, and himalaya-tui SHALL accept `email` and `display-name` in turn, the two binaries sharing one configuration file.

### Requirement: Account signature
An account MAY declare a `signature` and the `signature-delim` introducing it, and both MAY also be declared at the top level, where they apply to every account that does not override them.

`signature` SHALL be the signature alone. `signature-delim` SHALL default to the RFC 3676 §4.3 `"-- \n"` and SHALL be written verbatim, its own trailing newline included, so a delimiter meant to stand on its own line says so rather than relying on a rule. Both binaries SHALL assemble the block from the same two keys, so one configured value reads the same whichever composes.

### Requirement: Mailbox aliases
An account MAY map friendly mailbox names to backend-native ids under `[accounts.<name>.mailbox.alias]`. Alias names are case-insensitive on lookup and on storage. `-m/--mailbox` resolves an alias first, then a mailbox role, then passes the value verbatim; a role matching several mailboxes is an error. A shared command that omits `-m/--mailbox` resolves the `inbox` alias, else the `inbox` role. Account-level entries override same-named global entries, and ids are stored verbatim.

### Requirement: Config never written
Himalaya SHALL NOT persist config itself. The wizard prints a ready-to-save fragment on stdout; the user redirects it into their config file.

### Requirement: Path keys expand as the configuration is read
Every path-valued key SHALL expand `~` and environment variables while the configuration is deserialized, not where it is read. This covers `maildir.root`, `m2dir.root`, `pimdir.root`, the global and per-account `downloads-dir`, and the `tls.cert` of every backend.

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
Unknown keys SHALL stay tolerated, the file being shared with himalaya-tui. When no configured backend matches `--backend`, the error SHALL name the account and, under `auto`, the backends the command supports in this build; under a named backend, the missing block, or that the backend is unsupported. An account carrying the v1 `backend` table, which no v2 binary reads, SHALL load as if the table were absent, and that error SHALL name the table and point at MIGRATION.md.
