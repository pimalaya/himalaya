---
cairn: log
change: adopt-cairn
landed: 2026-07-25
---

# Adopt Cairn

Converted the ad-hoc docs/ folder into a Cairn root. The current, landed truth became seven spec capabilities: packaging (thin CLI over the io-* libraries), config (multi-account TOML schema), commands (the three command groups and backend selection), backends (the per-protocol adapter pattern and the shared operation set), search (the one query language and its per-backend translation), wizard (the discovery and account-setup flow), and provider-quirks (the real-provider facts the generic backends accommodate). The spec was seeded once from the src/main.rs header (the crate's architecture document) and the existing design notes, so it captures what Himalaya already does today.

The landed docs/io-email-inlining.md plan (dropping io-email for a local, per-backend dispatching client) is superseded by the backends capability and removed. The manual real-world provider test reports stay under docs/testing/ as an operational QA reference, outside the Cairn spec/changes/log convention.

The in-flight work became change proposals: v2-release (the release plumbing) and imap-special-use-aliases (the deferred RFC 6154 discovery, blocked upstream). This session's own wizard rework is recorded as a landed change. Defaults apply throughout, so no cairn.toml is needed; the cairn/ directory alone marks the root, and AGENTS.md carries the activation stanza.

This is a documentation reorganisation with no behaviour change.
