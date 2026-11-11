---
cairn: delta
change: wizard-auth-and-mailbox-aliases
---

## ADDED Requirements

### Requirement: One entry per service, then auth
The discovery list SHALL show one entry per reachable service, carrying the advertised authentication capabilities. The concrete method is chosen in a second, service-specific prompt (SASL mechanism for IMAP + SMTP, HTTP scheme for JMAP), skipped when only one qualifies.

### Requirement: OAuth folds into the API token
OAuth SHALL NOT be a standalone list entry. It folds into the API-token credential prompt, which offers the OS keyrings and the OAuth token brokers together, the brokers appearing only when the service advertises OAuth.

### Requirement: Per-protocol test and shared SMTP credentials
The IMAP + SMTP flow SHALL test IMAP first, ask whether SMTP reuses the same credentials, re-run the SASL prompt for a distinct one, and test SMTP last. A backend that validates itself inline skips the final account test.

### Requirement: Mailbox alias pre-fill
The wizard SHALL pre-fill `mailbox.alias.*`: JMAP roles read live, Gmail and Graph mapped from their fixed ids, IMAP pinned to the reserved `INBOX`.

### Requirement: Account name derived, not prompted
The wizard SHALL derive the account name from the input and use it as the table key, without a prompt.

## REMOVED Requirements

### Requirement: One entry per service and authentication method

### Requirement: OAuth entry loops back with a note
