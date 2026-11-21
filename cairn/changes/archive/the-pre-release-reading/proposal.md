---
cairn: change
id: the-pre-release-reading
status: landed
created: 2026-11-21
---

# The pre-release reading

## Why

The maintainer's reading of himalaya before v3 found the code and its README behind the family model neverest set in its v1 review (neverest cairn/changes/the-v1-review, follow-ups F2 and F4). src/shared/ mixed two roles: the unified API (titled so in its header) and helpers the protocol namespaces share, while the unified API also spread over src/email/, src/backend.rs and src/capability.rs, and the crate root already plays the shared role. 490 `//` lines remained against inline-004 (A9: all removed but `SAFETY`, the owner re-adding any by hand). The README (531 lines) broke readme-007, readme-008, readme-010 and readme-017, and its screenshot showed the v2 bordered table the text output no longer prints.

## What

- src/unified/ holds the whole unified API: the six command families, the client, the domain types (former src/email/), the `--backend` flag and the capability resolver. The helpers of src/shared/ move to the crate root. No printed byte changes.
- Every `//` line goes. A why worth keeping is compressed into the enclosing item's `///`, a test name or the spec (inline-008).
- tests/docs.rs, ported from neverest, holds the doc ceilings (inline-001, inline-002, clap items excepted per inline-006) and the README rules; the crate forbids `unsafe`.
- The README is rewritten on neverest's shape, `default = true` kept (most backends tie to one account, neverest is the exception), the screenshot replaced by an `envelope search --json` block with its real output, checked by tests/docs.rs against a maildir fixture: JSON is the family's direction, the CLIs managers rather than pure clients. GUIDELINES.md readme-006 is amended to allow that block for a CLI.
