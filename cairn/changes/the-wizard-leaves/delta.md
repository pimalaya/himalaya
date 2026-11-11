---
cairn: delta
change: the-wizard-leaves
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Missing account is an error
(config) When the config exists but lacks the requested account, the command SHALL fail with a hard error. When no configuration is found at all, a command needing one SHALL fail naming the path it looked at (the one `-c` or `HIMALAYA_CONFIG` gave, else the default location) and pointing at the documented sample, and a bare `himalaya` SHALL print the help. Himalaya SHALL NOT prompt to create a configuration.

### Requirement: One default account
(config) Resolving the default account SHALL fail, naming them, when several accounts are marked `default = true` once the configuration is loaded, layers and `-c` files merged. An account named with `-a` SHALL still resolve.

### Requirement: Config never written
(config) Himalaya SHALL NOT write a configuration file: the user writes it, from the documented sample.

### Requirement: The JSON contract is described and versioned
(commands) `json-schema` SHALL describe the failure object under the key `himalaya-error`, its `code` typed by the code vocabulary, and every command printing to stdout under its command key, checked by a test walking the command tree: only a bare `himalaya` (the help), `repl` and `mcp` are left out. A command printing one of two shapes SHALL be described by both, and a command printing events SHALL describe them under `<key>.events`. Schemas SHALL describe a value as printed, so a key always printed is required. Every generated schema SHALL carry an `$id` naming the contract version, the major version of Himalaya. Every registered payload SHALL be a JSON object, checked by a test. Piped `--json` output SHALL print one value per line.

### Requirement: Feature-gated backends
(packaging) Every backend SHALL sit behind its own cargo feature (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `m2dir`, `mbox`, `pimdir`), so a build ships only the protocols it needs. A protocol command and its backend adapter compile only when its feature is on.

### Requirement: Bearer-only proprietary APIs
(provider-quirks) The Gmail and Microsoft Graph REST APIs SHALL be authenticated with a single OAuth 2.0 bearer token only; neither accepts an app password. Their account config carries one token field.

### Requirement: SASL OAuth carries a username
(provider-quirks) The IMAP and SMTP SASL OAuth mechanisms SHALL carry the login, not just the token: `XOAUTH2` encodes `user=<login>` and `OAUTHBEARER` encodes the login as the GS2 authorization identity, so their configuration names a login. JMAP over HTTP `Authorization: Bearer` is token-only and takes no login.

### Requirement: A session runs the CLI over one account
(repl) `himalaya repl` SHALL start a session bound to one account and one `--backend`, read one command per stdin line, split as a shell splits it, parse it with the one-shot grammar and run it, and loop until `quit`, `exit` or EOF. `repl` and `mcp` SHALL be refused inside it. A parse error or a command error SHALL be printed, on stdout as the failure object when piped, and the loop SHALL continue.

## REMOVED Requirements

The wizard capability leaves whole, spec/wizard.md with it: Himalaya no longer generates a configuration.

### Requirement: Input orients the flow
### Requirement: Discovery is time-bounded
### Requirement: Each domain gets its best option without a question
### Requirement: The review lists each pick in plain words
### Requirement: Nothing discovered asks for a server URL
### Requirement: A typed server URL names its protocol by its scheme
### Requirement: The sign-in mechanism is picked, not asked
### Requirement: Sign-ins are grouped by identity
### Requirement: A password is typed once, tested, then stored
### Requirement: OAuth hands off to Ortie
### Requirement: A refusal offers a way on
### Requirement: No mailbox alias pre-fill
### Requirement: The prompted address is kept
### Requirement: Account name derived, not prompted
### Requirement: Connection tested before printing
### Requirement: Local backend auto-detected
### Requirement: A named command runs the wizard
### Requirement: The offer is a hook, not a gate
### Requirement: The welcome names the missing path
### Requirement: Generating never rewrites what a human wrote
### Requirement: A generated account reads in a deliberate order
### Requirement: Generated configuration files are private
