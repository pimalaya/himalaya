---
cairn: spec
capability: wizard
status: current
---

# Wizard

Bare `himalaya` (no subcommand) runs the interactive configuration wizard, and it is also proposed when a command finds no config at all. The wizard discovers an account and prints it as a ready-to-save TOML fragment on stdout, writing nothing to disk. Prompts render on stderr, so redirecting stdout into a config file works directly.

### Requirement: Input orients the flow
A single prompt SHALL accept an email address (or bare domain), a `scheme://` server URL, or a local folder path. An address runs io-pim-discovery's parallel discovery; a URL is configured by hand; a folder is a local Maildir or m2dir.

### Requirement: One entry per service, then auth
The discovery list SHALL show one entry per reachable service (IMAP + SMTP, JMAP, Gmail, Microsoft Graph), carrying the authentication capabilities the service advertised. After a service is picked, the authentication method SHALL be chosen in a second, service-specific prompt, skipped when only one method qualifies: the SASL mechanism (`PLAIN`, `LOGIN`, `SCRAM-SHA-256`, `OAUTHBEARER`, `XOAUTH2`, `ANONYMOUS`) for IMAP + SMTP, the HTTP scheme (Basic or Bearer) for JMAP. A detected Google or Microsoft account collapses to its dedicated set.

### Requirement: OAuth folds into the API token
Himalaya runs no OAuth 2.0 grant itself, so OAuth SHALL NOT be a standalone list entry. It folds into the API-token credential prompt, which offers the OS keyrings (for a token the user generated) and the OAuth token brokers (Ortie, pizauth, oama) together, the brokers appearing only when the service advertises OAuth.

### Requirement: Per-protocol test and shared SMTP credentials
The IMAP + SMTP flow SHALL test each protocol as it configures it: the IMAP connection is validated first, then the wizard asks whether SMTP reuses the same credentials (the two may advertise different auth), re-running the SASL prompt for a distinct one, and tests SMTP last. JMAP and the proprietary APIs are validated by the account test. A backend that validates itself inline skips the final account test.

### Requirement: Mailbox alias pre-fill
The wizard SHALL pre-fill `mailbox.alias.*` so a generated account has a working default mailbox and known special-use targets. JMAP reads the RFC 8621 mailbox roles live over the tested connection. Gmail and Microsoft Graph map their fixed system-label ids (`INBOX`, `SENT`, ...) and well-known folder names (`inbox`, `sentitems`, ...). IMAP pins only the reserved `INBOX`; the other IMAP special-use roles are not discovered yet (see provider-quirks).

### Requirement: Account name derived, not prompted
The wizard SHALL NOT prompt for an account name. It derives one from the input (the domain's first label, or the folder name) and uses it as the `[accounts.<name>]` table key; the user renames it by editing that key.

### Requirement: Connection tested before printing
The account's connection SHALL be tested before the fragment is printed, so a bad credential or endpoint stops the wizard instead of yielding a config that cannot connect. The printed fragment is compact: only the `[accounts.<name>]` table stays a section header, other tables flatten into dotted keys, and empty tables and defaulted values are dropped.
