---
cairn: change
id: wizard-discovery-only
status: landed
created: 2026-07-30
---

# Wizard is discovery-only; stop when nothing is discovered

## Why
The wizard grew a hand-entry configuration path: when discovery found nothing (`manual_fallback`), and whenever the user typed a `scheme://` server URL (`configure_server` → `imap_smtp::configure_manual` / `jmap::configure_manual`), it walked the user through prompting every field (host, port, encryption, SMTP endpoint) by hand. This is a large, complex surface that duplicates what a hand-written config file already does better, and it makes the wizard responsible for building configs it cannot validate against a provider.

The wizard should do one thing: help find the right config *automatically*. Interaction stays minimal, and the output is a pre-configured, tested account. When it cannot discover a config from the given email, domain or server URL, it should stop and tell the user to write the config by hand, pointing at the documented sample — not fall into a manual builder.

## What
Unify every non-local input onto the discovery flow and delete all hand-entry.

- **Input.** A local folder path (or a `file://` URL) stays a local backend and continues the flow: the wizard auto-detects the store kind — a `.m2store`/`.m2dir` marker means m2dir, a `cur`/`new`/`tmp` tree means Maildir — and only prompts Maildir-vs-m2dir when the directory is empty or ambiguous. Everything else — an email, a bare domain, or a `scheme://` server URL — runs io-pim-discovery's parallel discovery. A URL discovers from its host and its scheme narrows the results (see mapping below).
- **Stop on empty.** When discovery yields no supported configuration (including when the deadline passes with nothing completed, and when a URL's scheme filter leaves nothing), the wizard stops with a clear message: it could not automatically discover a configuration for the given input, and the user should write the account by hand using the sample as a starting point (`https://github.com/pimalaya/himalaya/blob/master/config.sample.toml`). No prompts, no partial account.
- **Success is unchanged.** A discovered entry is configured exactly as today: pick the service (when several), pick the SASL mechanism / HTTP scheme, enter the credential, and the connection is tested before the fragment is printed. This interaction is already minimal and stays.

### URL scheme mapping (for review)
A `scheme://host[...]` URL discovers from `host` (as `@host`), then keeps only the discovered entries matching the scheme:

| Scheme | Kept service | Security constraint |
| --- | --- | --- |
| `imap` | IMAP + SMTP | none (any IMAP security discovered) |
| `imaps` | IMAP + SMTP | IMAP endpoint must be implicit TLS |
| `jmap` / `jmaps` / `http` / `https` | JMAP | none (JMAP is always HTTPS) |

Proprietary entries (Gmail, Microsoft Graph) are dropped when a scheme is given, since the user asked for a specific open protocol. If the filter leaves nothing, the wizard stops as above.

- **No invented hosts.** IMAP is already never guessed (no IMAP discovered ⇒ no entry). SMTP SHALL follow the same rule: `default_smtp` (guessing `smtp.<domain>`) is removed. When IMAP is discovered but SMTP is not, the wizard produces an IMAP-only account (no `smtp` block, no SMTP test) rather than inventing and then testing a host that likely fails — the very failure behind #722. The user adds SMTP by hand, like anything else undiscovered.

## Removed
- `discover.rs`: `manual_fallback` (both cfg variants), `split_email`, and `configure_server`'s hand-entry routing (folded into the unified discovery flow with scheme filtering).
- `imap_smtp.rs`: `configure_manual` and its helpers not shared with the discovered flow (`prompt_smtp_endpoint`), and `default_smtp`.
- `jmap.rs`: `configure_manual`.

## Non-goals
- The local backend is still asked when detection is inconclusive; auto-detection only removes the prompt when the on-disk markers are unambiguous.
