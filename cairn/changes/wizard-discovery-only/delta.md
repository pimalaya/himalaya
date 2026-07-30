---
cairn: delta
change: wizard-discovery-only
---

## MODIFIED Requirements

### Requirement: Input orients the flow
A single prompt SHALL accept an email address (or bare domain), a `scheme://` server URL, or a local folder path. An email, bare domain or server URL runs io-pim-discovery's parallel discovery; a folder is a local Maildir or m2dir. A server URL discovers from its host and its scheme narrows the discovered entries: `imap`/`imaps` keep IMAP + SMTP (with `imaps` requiring an implicit-TLS IMAP endpoint), and `jmap`/`jmaps`/`http`/`https` keep JMAP; proprietary entries (Gmail, Microsoft Graph) are dropped when a scheme is given. The wizard SHALL NOT offer any hand-entry of server fields.

### Requirement: Discovery is time-bounded
The parallel discovery run SHALL be bounded by a short deadline so a single unreachable endpoint (a firewalled port, a black-hole host) cannot stall the interactive wizard. Each mechanism runs independently; any that has not reported by the deadline is abandoned, and only what completed in time is offered. When nothing completes, the wizard stops (see "Stop when nothing is discovered").

### Requirement: One entry per service, then auth
The discovery list SHALL show one entry per reachable service (IMAP + SMTP, JMAP, Gmail, Microsoft Graph). After a service is picked, the authentication method SHALL be chosen in a second, service-specific prompt, skipped when only one method qualifies. For IMAP the wizard SHALL first probe the server's live CAPABILITY over an unauthenticated connection and offer only the SASL mechanisms it advertises, most preferred first and the legacy `LOGIN` command last; a server exposing no SASL AUTH and no LOGINDISABLED therefore offers `LOGIN` alone. On any probe failure the wizard SHALL log the error and fall back to the full mechanism list (`PLAIN`, `LOGIN`, `SCRAM-SHA-256`, `OAUTHBEARER`, `XOAUTH2`, `ANONYMOUS`), never stopping. SMTP SHALL keep the discovery-advertised list, since it negotiates auth over EHLO rather than the IMAP probe. JMAP uses the HTTP scheme (Basic or Bearer). A detected Google or Microsoft account collapses to its dedicated set.

### Requirement: Per-protocol test and shared SMTP credentials
The discovered IMAP + SMTP flow SHALL test each protocol as it configures it: the IMAP connection is validated first, then, when an SMTP endpoint was discovered, the wizard asks whether SMTP reuses the same credentials (the two may advertise different auth), re-running the SASL prompt for a distinct one, and tests SMTP last. The wizard SHALL NOT invent an SMTP host: when discovery found IMAP but no SMTP, it produces an IMAP-only account (no `smtp` block, no SMTP test) instead of guessing `smtp.<domain>`. IMAP is likewise never guessed. JMAP and the proprietary APIs are validated by the account test. A backend that validates itself inline skips the final account test.

## ADDED Requirements

### Requirement: Local backend auto-detected
A typed folder path or `file://` URL SHALL configure a local backend, auto-detecting the store kind from on-disk markers: a `.m2store` or `.m2dir` marker means m2dir, a `cur`/`new`/`tmp` tree means Maildir. The wizard SHALL prompt Maildir-vs-m2dir only when both backends are compiled in and detection is inconclusive (an empty or ambiguous directory).

### Requirement: Stop when nothing is discovered
When discovery yields no supported configuration for the given input — an empty result, the deadline elapsing with nothing completed, or a URL scheme filter leaving no entry — the wizard SHALL stop with a message stating it could not automatically discover a configuration for the input, and inviting the user to write the account by hand using the documented sample configuration (linked). It SHALL NOT prompt for any server field or emit a partial account. The wizard performs no hand-entry configuration of remote accounts.
