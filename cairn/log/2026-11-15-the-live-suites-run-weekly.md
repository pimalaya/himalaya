---
cairn: log
change: the-live-suites-run-weekly
landed: 2026-11-15
---

# The live suites run weekly

Line N0 of the family's cli-alignment-2026-11-15 plan: the credentialed live suites leave push.

## What landed

- .github/workflows/tests.yml: `on` gains `workflow_dispatch` and `schedule` (`0 1 * * 1`, Mondays at 01:00 UTC, Himalaya's slot in the family); the `live` job runs only when `github.event_name` is `workflow_dispatch` or `schedule`; `tests` and `features` unchanged. Checked with actionlint.
- CONTRIBUTING.md: "Continuous integration" says what runs on a push, when the live suites run, `gh workflow run tests.yml`, and that a release runs them on the commit it tags.

## Capabilities moved

- [packaging](../spec/packaging.md): *Live provider suites* runs them weekly and by hand, never on a push.
