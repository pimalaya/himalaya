---
cairn: log
change: the-pre-release-reading
landed: 2026-11-21
---

# The pre-release reading

The maintainer's reading of himalaya before v3, aligning it on the model neverest set in its v1 review (follow-ups F2 and F4).

## What landed

- b3e15438: src/unified/ holds the whole unified API (the six command families, the client, the domain types of the former src/email/, the `--backend` flag, the capability resolver); the helpers src/shared/ held move to the crate root. No printed byte changes; himalaya-tui follows the paths when it bumps its pin.
- 4c026b28: the 491 `//` lines go, a why that mattered compressed into its item's `///` or into the spec.
- aa32c77f: the 484 doc blocks over inline-001 and inline-002 are cut, held by tests/docs.rs (ported from neverest, clap items aside); both crate roots forbid `unsafe`.
- ce5ad83c: `Envelope` drops its kebab-case aliases, dead since the mbox cache version 4.
- README cut from 531 to 304 lines on neverest's shape: eight Features, JSON first; four installation entries (the Arch, Homebrew, Scoop and Fedora ones dropped per readme-010); Configuration as paths and the six provider recipes; Usage as one shell block. The screenshot of the v2 table goes, replaced by an `envelope search --json` block whose output tests/docs.rs checks against a Maildir fixture. `default = true` stays: most backends tie to one account, neverest being the exception.
- .github GUIDELINES.md readme-003 and readme-006 (b5be1f1): a CLI shows that tested terminal block instead of a screenshot.

## Capabilities moved

- [backends](../spec/backends.md), [commands](../spec/commands.md), [search](../spec/search.md), [repl](../spec/repl.md), [mcp](../spec/mcp.md): the rules the deleted comments carried, and two sentences the code contradicted (a pimdir role, a JMAP `before`).
