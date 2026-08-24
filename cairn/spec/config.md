---
cairn: spec
capability: config
status: current
---

# Config

Himalaya is configured through TOML: a top-level block plus named account blocks, one table per account under `[accounts.<name>]`, each carrying optional per-backend sub-blocks. The config schema is a set of pure DTOs (`*Config` types) mirroring the nested TOML shape; the selected account is flattened into a runtime `Account` view that commands consume. Config files stay user-owned: Himalaya never writes them.

### Requirement: Multi-account schema
The config SHALL be multi-account: a top-level block holding shared defaults, plus `[accounts.<name>]` blocks. Each account carries at most one storage backend sub-block (`imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `m2dir`) and optional service sub-blocks for `smtp` and `sieve`.

The `sieve` block SHALL accept `sieve://`, `sieves://`, and `unix://`
servers, default bare authorities to implicit TLS on port 4190, and expose
the shared TLS vocabulary plus optional SASL LOGIN or PLAIN credentials.
STARTTLS SHALL only be enabled explicitly on a `sieve://` endpoint.

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
An account MAY map friendly mailbox names to backend-native ids under `[accounts.<name>.mailbox.alias]`. Alias names are case-insensitive on lookup and on storage. The entry named `inbox` is the implicit default mailbox: a shared command that omits `-m/--mailbox` resolves it. Account-level entries override same-named global entries, and ids are stored verbatim.

### Requirement: Config never written
Himalaya SHALL NOT persist config itself. The wizard prints a ready-to-save fragment on stdout; the user redirects it into their config file.
