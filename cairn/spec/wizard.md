---
cairn: spec
capability: wizard
status: current
---

# Wizard

Bare `himalaya` (no subcommand) runs the interactive configuration wizard, and it is also proposed when a command finds no config at all. The wizard discovers an account and prints it as a ready-to-save TOML fragment on stdout, writing nothing to disk. Prompts render on stderr, so redirecting stdout into a config file works directly.

### Requirement: Input orients the flow
A single prompt SHALL accept an email address (or bare domain), a `scheme://` server URL, or a local folder path. An email, bare domain or server URL runs io-pim-discovery's parallel discovery; a folder is a local Maildir or m2dir. A server URL discovers from its host and its scheme narrows the discovered entries: `imap`/`imaps` keep IMAP + SMTP (with `imaps` requiring an implicit-TLS IMAP endpoint), and `jmap`/`jmaps`/`http`/`https` keep JMAP; proprietary entries (Gmail, Microsoft Graph) are dropped when a scheme is given. The wizard SHALL NOT offer any hand-entry of server fields.

### Requirement: Discovery is time-bounded
The parallel discovery run SHALL be bounded by a short deadline so a single unreachable endpoint (a firewalled port, a black-hole host) cannot stall the interactive wizard. Each mechanism runs independently; any that has not reported by the deadline is abandoned, and only what completed in time is offered. When nothing completes, the wizard stops (see "Stop when nothing is discovered").

### Requirement: One entry per service, then auth
The discovery list SHALL show one entry per reachable service (IMAP + SMTP, JMAP, Gmail, Microsoft Graph). After a service is picked, the authentication method SHALL be chosen in a second, service-specific prompt, skipped when only one method qualifies. For IMAP the wizard SHALL first probe the server's live CAPABILITY over an unauthenticated connection and offer only the SASL mechanisms it advertises, most preferred first and the legacy `LOGIN` command last; a server exposing no SASL AUTH and no LOGINDISABLED therefore offers `LOGIN` alone. On any probe failure the wizard SHALL log the error and fall back to the full mechanism list (`PLAIN`, `LOGIN`, `SCRAM-SHA-256`, `OAUTHBEARER`, `XOAUTH2`, `ANONYMOUS`), never stopping. SMTP SHALL keep the discovery-advertised list, since it negotiates auth over EHLO rather than the IMAP probe. JMAP uses the HTTP scheme (Basic or Bearer). A detected Google or Microsoft account collapses to its dedicated set.

### Requirement: OAuth folds into the API token
Himalaya runs no OAuth 2.0 grant itself, so OAuth SHALL NOT be a standalone list entry. It folds into the API-token credential prompt, which offers the OS keyrings (for a token the user generated) and the OAuth token brokers (Ortie, pizauth, oama) together, the brokers appearing only when the service advertises OAuth.

### Requirement: Per-protocol test and shared SMTP credentials
The discovered IMAP + SMTP flow SHALL test each protocol as it configures it: the IMAP connection is validated first, then, when an SMTP endpoint was discovered, the wizard asks whether SMTP reuses the same credentials (the two may advertise different auth), re-running the SASL prompt for a distinct one, and tests SMTP last. The wizard SHALL NOT invent an SMTP host: when discovery found IMAP but no SMTP, it produces an IMAP-only account (no `smtp` block, no SMTP test) instead of guessing `smtp.<domain>`. IMAP is likewise never guessed. JMAP and the proprietary APIs are validated by the account test. A backend that validates itself inline skips the final account test.

### Requirement: Mailbox alias pre-fill
The wizard SHALL pre-fill `mailbox.alias.*` so a generated account has a working default mailbox and known special-use targets. JMAP reads the RFC 8621 mailbox roles live over the tested connection. Gmail and Microsoft Graph map their fixed system-label ids (`INBOX`, `SENT`, ...) and well-known folder names (`inbox`, `sentitems`, ...). IMAP pins only the reserved `INBOX`; the other IMAP special-use roles are not discovered yet (see provider-quirks).

### Requirement: Account name derived, not prompted
The wizard SHALL NOT prompt for an account name. It derives one from the input (the domain's first label, or the folder name) and uses it as the `[accounts.<name>]` table key; the user renames it by editing that key.

### Requirement: Connection tested before printing
The account's connection SHALL be tested before the fragment is printed, so a bad credential or endpoint stops the wizard instead of yielding a config that cannot connect. The printed fragment is compact: only the `[accounts.<name>]` table stays a section header, other tables flatten into dotted keys, and empty tables and defaulted values are dropped.

### Requirement: Stop when nothing is discovered
When discovery yields no supported configuration for the given input — an empty result, the deadline elapsing with nothing completed, or a URL scheme filter leaving no entry — the wizard SHALL stop with a message stating it could not automatically discover a configuration for the input, and inviting the user to write the account by hand using the documented sample configuration (linked). It SHALL NOT prompt for any server field or emit a partial account. The wizard performs no hand-entry configuration of remote accounts.

### Requirement: Local backend auto-detected
A typed folder path or `file://` URL SHALL configure a local backend, auto-detecting the store kind from on-disk markers: a `.m2store` or `.m2dir` marker means m2dir, a `cur`/`new`/`tmp` tree means Maildir. The wizard SHALL prompt Maildir-vs-m2dir only when both backends are compiled in and detection is inconclusive (an empty or ambiguous directory).

### Requirement: A named command runs the wizard
A `configure` command (alias `wizard`) SHALL run the wizard by name, without the welcome, since whoever typed it knows what it does. It refuses to run when stdin is not a terminal, naming the sample configuration to write by hand instead.

### Requirement: The offer is a hook, not a gate
A missing configuration SHALL raise an offer to generate one, from a bare invocation and from any command needing an account. The offer never ends the process: a command carries on afterwards either way, so accepting gives it a chance to work and declining leaves it to fail on the configuration it still has not got. A bare invocation has nothing to carry on to, so a declined offer falls back to the help. Nothing is offered when stdin is not a terminal or `--json` is set.

### Requirement: The welcome names the missing path
The welcome SHALL name the configuration path that was looked for, which is the one `-c` or `HIMALAYA_CONFIG` gave or the default location, so a mistyped path shows up as itself rather than as a generic first run. It frames the product, points at the documented sample, and names the command that runs the wizard again later.

### Requirement: Generating never rewrites what a human wrote
The wizard SHALL write a configuration file that does not exist and append a plain text block to one that does, never parsing and re-serializing the document, so comments, ordering and formatting survive. Two invariants guard the append: the account name must be free, since a second `[accounts.<name>]` table makes the whole document fail to parse, and the generated account claims `default` only when no other account does. The derived name is suffixed until free. The target path is not prompted: it is where `-c` pointed, or the default location.

### Requirement: A generated account reads in a deliberate order
The serializer SHALL decide what a generated account holds, so a defaulted field is omitted and no field is enumerated twice, but the rendering SHALL order what it emits: the groups run most-defining first (`default`, the storage backend, the transport, the mailboxes, the rendering options), an unrecognised group renders after them rather than being dropped, a group's `server` key reads before the credentials qualifying it, and a blank line separates groups.
