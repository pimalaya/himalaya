---
cairn: log
change: testing-into-cairn
landed: 2026-07-25
---

# Move testing reports into cairn

The [adopt-cairn](2026-07-25-adopt-cairn.md) migration kept the manual
provider test reports under `docs/testing/`, deliberately outside the
Cairn convention. That is now reversed: the whole corpus moved to
`cairn/spec/testing/` and the `docs/` folder was deleted, so Cairn is the
single home for both design memory and operational QA.

The reports live under `spec/` (current truth) rather than `log/`: the
testing capability is *how* QA is run (the followable
`provider-test-plan.md`) and *what* is currently covered (the per-backend
report index), which is standing truth about the project, not a dated
event. The index (`cairn/spec/testing/README.md`) gained the standard
`capability: testing` spec frontmatter; the individual reports and the
plan moved verbatim.

References updated: the `src/main.rs` crate header and the
`v2-release` change (proposal + tasks) now point at
`cairn/spec/testing/`.
