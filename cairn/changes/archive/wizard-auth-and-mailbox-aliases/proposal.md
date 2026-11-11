---
cairn: change
id: wizard-auth-and-mailbox-aliases
status: landed
created: 2026-07-25
---

# Wizard auth model and mailbox alias pre-fill

## Why
The discovered-account wizard split each service into one list entry per authentication method, so a JMAP provider like Fastmail showed a redundant "API token" and "OAuth 2.0" row for the same endpoint, and the OAuth row was a dead end (Himalaya runs no grant). IMAP + SMTP were configured with a single hardcoded credential kind and no per-protocol validation, even though the two sides can advertise different auth. And a generated account carried no `mailbox.alias.*`, so the first shared command failed with "Mailbox is required".

## What
Restructure the discovered flow into a two-step selection and pre-fill the aliases.

The discovery list shows one entry per reachable service. The authentication method is then chosen in a second, service-specific prompt: the SASL mechanism for IMAP + SMTP, the HTTP scheme for JMAP, skipped when only one qualifies. OAuth stops being a list entry: it folds into the API-token credential prompt, which now combines the OS keyrings with the OAuth token brokers, the brokers shown only when the service advertises OAuth.

IMAP + SMTP are tested as they are configured: IMAP first, then a question of whether SMTP reuses the same credentials, re-running the SASL prompt otherwise, then SMTP. The wizard pre-fills `mailbox.alias.*`: JMAP roles read live, Gmail and Graph mapped from their fixed ids, IMAP pinned to `INBOX`. The account-name prompt is dropped; the name is derived from the input and renamed by editing the printed table key.
