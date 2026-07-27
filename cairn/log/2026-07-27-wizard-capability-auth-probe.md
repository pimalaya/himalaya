---
cairn: log
change: wizard-capability-auth-probe
landed: 2026-07-27
---

# Wizard probes IMAP capabilities and bounds discovery

Taught the wizard to ask the server what it accepts instead of guessing. Before the SASL prompt it now opens an unauthenticated IMAP connection, reads CAPABILITY, and offers only the advertised mechanisms, most preferred first and the legacy `LOGIN` command last. A perdition-style proxy advertising a bare `IMAP4 IMAP4REV1` (isae-supaero.fr) now offers `LOGIN` alone, where the old wizard defaulted to `AUTHENTICATE PLAIN` and failed. Both the discovered and the manually entered IMAP paths probe; the manual path no longer hardcodes PLAIN. On any probe failure the wizard logs the error and falls back to the full mechanism list rather than stopping. SMTP keeps its discovery-advertised list, since it negotiates auth over EHLO.

Discovery is now time-bounded: a single unreachable endpoint no longer stalls the wizard for the operating-system connect timeout. Each mechanism runs on its own thread and any still running at the deadline is abandoned.

The moved capability forward on the wizard: two requirements changed (the IMAP auth prompt is now capability-driven, discovery is time-bounded). Supporting library work landed in io-imap (`rfc3501::capability::available_auth_mechanisms`, which reuses pimalaya-stream's existing `SaslMechanism` tags rather than a new enum and lives in the coroutine core so it is not TLS-provider gated, plus decoupling an unrelated per-read timeout that broke slow commands) and io-pim-discovery (`compose_all_within`). Neither library uses Cairn, so their history lives in their own CHANGELOGs.
