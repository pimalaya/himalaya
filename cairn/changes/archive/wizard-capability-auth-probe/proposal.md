---
cairn: change
id: wizard-capability-auth-probe
status: landed
created: 2026-07-27
---

# Wizard probes IMAP capabilities and bounds discovery

## Why
The wizard offered SASL mechanisms without asking the server what it accepts, defaulting to PLAIN. A perdition-style proxy (isae-supaero.fr) advertises a bare `IMAP4 IMAP4REV1` with no SASL AUTH and only the legacy `LOGIN` command, so `AUTHENTICATE PLAIN` failed with "NO mechanism not supported". The manual path was worse: it hardcoded PLAIN and never asked. Separately, discovery waited for every mechanism thread to finish, so a single black-hole endpoint (a firewalled port, an unreachable host) stalled the wizard for the operating-system connect timeout.

## What
Probe the server's live capability before offering mechanisms, and time-box discovery.

Before the SASL prompt the wizard opens an unauthenticated IMAP connection, reads CAPABILITY, and offers only the mechanisms the server advertises, most preferred first and the legacy `LOGIN` command last. A server exposing no SASL AUTH and no LOGINDISABLED yields just `LOGIN`, which auto-selects. Both the discovered and the manually entered IMAP paths probe; on any probe failure the wizard logs the error and falls back to the full mechanism list, never stopping. SMTP keeps its discovery-advertised list, since it negotiates auth over EHLO rather than the IMAP probe.

Discovery is bounded by a short deadline: each mechanism runs on its own thread and any still running at the deadline is abandoned, so an unreachable endpoint no longer stalls the wizard. The capability-to-mechanism mapping lives in io-imap (`available_auth_mechanisms`); the deadline lives in io-pim-discovery (`compose_all_within`).
