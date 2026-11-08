---
cairn: change
id: the-wizard-picks-and-asks-once
status: landed
created: 2026-11-08
---

# The wizard picks, then asks once

## Why

The wizard asks what discovery already knows. After the address it lists every reachable service and asks which one, then asks the SASL mechanism, the login, a keyring strategy and an entry name per credential, then whether SMTP reuses the IMAP credentials. A password is never typed into the wizard: the user stores it beforehand under a name they have to guess, and a wrong guess surfaces only as a failed test. OAuth is a broker list behind the API token prompt, with nothing signing in.

When discovery finds nothing, the wizard stops and points at the sample, even for a user who knows the server's URL.

The Pimalaya Android app's onboarding settled a shorter flow, approved by the owner for the CLIs on 2026-11-08 (flow model: the Android onboarding spec and its one-onboarding-path change): one input, the best option picked per domain, a review in plain words, one sign-in per identity, a password typed once, tested and stored by the wizard. Himalaya is the reference cardamum and calendula mirror.

## What

- One input, nothing asked before discovery. io-pim-discovery moves from 0.9 to 0.11 (MX provider fallback for autoconfig), its provider rules now asked for the Gmail API and Microsoft Graph services they carry.
- Each domain, mail and sending, gets the best option found without a question: the provider's own API where a provider rule matched, JMAP before IMAP, implicit TLS before STARTTLS, OAuth first only when `ortie` is installed. Sending goes through mail when mail reads over JMAP, the Gmail API or Graph, else over the best SMTP server, else nowhere. Only encrypted servers are offered.
- A review through pimalaya-cli's `review::prompt`, one row per domain in plain words, each changeable to Auto, any option found, a custom server URL, and for sending "Through mail" or "No sending".
- Nothing discovered asks `Server URL:`, the scheme naming the protocol (`imaps`, `imap`, `smtps`, `smtp`, `https`, `jmap(s)`), `http://` refused; an empty answer stops as before. A typed IMAP server asks the sending server when none was discovered.
- No SASL mechanism, HTTP scheme, login (when known) or API user id prompt: the IMAP CAPABILITY probe picks the mechanism.
- Sign-ins grouped by identity: one OAuth hand-off to Ortie per authorization server (the broker list when Ortie is missing), one password per group of servers of one name, typed once through `keyring::prompt_password`, tested at once, then stored through `keyring::prompt_store` under `himalaya-<account>-<service>`.
- A refusal offers Try again, Change the login, Leave the domain out, Back to the review.
- `Your name (optional):` last, written as `display-name`.
- The save, append and print rules, the local folder flow, the name derivation, the deadline and the welcome stay as they are. No configuration key is added (himalaya-tui shares the format).

Spec: wizard.
