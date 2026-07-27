---
cairn: spec
capability: wizard
status: current
---

# Wizard

Bare `himalaya` (no subcommand) runs the interactive configuration wizard, and it is also proposed when a command finds no config at all. The wizard discovers an account and prints it as a ready-to-save TOML fragment on stdout, writing nothing to disk. Prompts render on stderr, so redirecting stdout into a config file works directly.

### Requirement: Input orients the flow
A single prompt SHALL accept an email address (or bare domain), a `scheme://` server URL, or a local folder path. An address runs io-pim-discovery's parallel discovery; a URL is configured by hand; a folder is a local Maildir or m2dir.

### Requirement: Discovery is time-bounded
The parallel discovery run SHALL be bounded by a short deadline so a single unreachable endpoint (a firewalled port, a black-hole host) cannot stall the interactive wizard. Each mechanism runs independently; any that has not reported by the deadline is abandoned, and only what completed in time is offered. When nothing completes, the wizard proceeds as if discovery found nothing and falls back to manual entry.

### Requirement: One entry per service, then auth
The discovery list SHALL show one entry per reachable service (IMAP + SMTP, JMAP, Gmail, Microsoft Graph). After a service is picked, the authentication method SHALL be chosen in a second, service-specific prompt, skipped when only one method qualifies. For IMAP the wizard SHALL first probe the server's live CAPABILITY over an unauthenticated connection and offer only the SASL mechanisms it advertises, most preferred first and the legacy `LOGIN` command last; a server exposing no SASL AUTH and no LOGINDISABLED therefore offers `LOGIN` alone. The manually entered IMAP path SHALL probe the same way instead of assuming a mechanism. On any probe failure the wizard SHALL log the error and fall back to the full mechanism list (`PLAIN`, `LOGIN`, `SCRAM-SHA-256`, `OAUTHBEARER`, `XOAUTH2`, `ANONYMOUS`), never stopping. SMTP SHALL keep the discovery-advertised list, since it negotiates auth over EHLO rather than the IMAP probe. JMAP uses the HTTP scheme (Basic or Bearer). A detected Google or Microsoft account collapses to its dedicated set.

### Requirement: OAuth folds into the API token
Himalaya runs no OAuth 2.0 grant itself, so OAuth SHALL NOT be a standalone list entry. It folds into the API-token credential prompt, which offers the OS keyrings (for a token the user generated) and the OAuth token brokers (Ortie, pizauth, oama) together, the brokers appearing only when the service advertises OAuth.

### Requirement: Per-protocol test and shared SMTP credentials
The discovered IMAP + SMTP flow SHALL test each protocol as it configures it: the IMAP connection is validated first, then the wizard asks whether SMTP reuses the same credentials (the two may advertise different auth), re-running the SASL prompt for a distinct one, and tests SMTP last. The manual IMAP + SMTP flow SHALL likewise ask whether to reuse the IMAP credentials for SMTP: when accepted it reuses the IMAP SASL and prompts only the SMTP endpoint (host seeded from the IMAP host, encryption, port); when declined it runs the full SMTP prompts. JMAP and the proprietary APIs are validated by the account test. A backend that validates itself inline skips the final account test.

### Requirement: Mailbox alias pre-fill
The wizard SHALL pre-fill `mailbox.alias.*` so a generated account has a working default mailbox and known special-use targets. JMAP reads the RFC 8621 mailbox roles live over the tested connection. Gmail and Microsoft Graph map their fixed system-label ids (`INBOX`, `SENT`, ...) and well-known folder names (`inbox`, `sentitems`, ...). IMAP pins only the reserved `INBOX`; the other IMAP special-use roles are not discovered yet (see provider-quirks).

### Requirement: Account name derived, not prompted
The wizard SHALL NOT prompt for an account name. It derives one from the input (the domain's first label, or the folder name) and uses it as the `[accounts.<name>]` table key; the user renames it by editing that key.

### Requirement: Connection tested before printing
The account's connection SHALL be tested before the fragment is printed, so a bad credential or endpoint stops the wizard instead of yielding a config that cannot connect. The printed fragment is compact: only the `[accounts.<name>]` table stays a section header, other tables flatten into dotted keys, and empty tables and defaulted values are dropped.
