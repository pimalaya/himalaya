---
cairn: delta
change: wizard-capability-auth-probe
---

## MODIFIED Requirements

### Requirement: One entry per service, then auth
The discovery list SHALL show one entry per reachable service (IMAP + SMTP, JMAP, Gmail, Microsoft Graph). After a service is picked, the authentication method SHALL be chosen in a second, service-specific prompt, skipped when only one method qualifies. For IMAP the wizard SHALL first probe the server's live CAPABILITY over an unauthenticated connection and offer only the SASL mechanisms it advertises, most preferred first and the legacy `LOGIN` command last; a server exposing no SASL AUTH and no LOGINDISABLED therefore offers `LOGIN` alone. The manually entered IMAP path SHALL probe the same way instead of assuming a mechanism. On any probe failure the wizard SHALL log the error and fall back to the full mechanism list (`PLAIN`, `LOGIN`, `SCRAM-SHA-256`, `OAUTHBEARER`, `XOAUTH2`, `ANONYMOUS`), never stopping. SMTP SHALL keep the discovery-advertised list, since it negotiates auth over EHLO rather than the IMAP probe. JMAP uses the HTTP scheme (Basic or Bearer). A detected Google or Microsoft account collapses to its dedicated set.

## ADDED Requirements

### Requirement: Discovery is time-bounded
The parallel discovery run SHALL be bounded by a short deadline so a single unreachable endpoint (a firewalled port, a black-hole host) cannot stall the interactive wizard. Each mechanism runs independently; any that has not reported by the deadline is abandoned, and only what completed in time is offered. When nothing completes, the wizard proceeds as if discovery found nothing and falls back to manual entry.
