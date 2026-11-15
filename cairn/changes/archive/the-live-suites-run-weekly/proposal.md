---
cairn: change
id: the-live-suites-run-weekly
status: landed
created: 2026-11-15
---

# The live suites run weekly

## Why

Line N0 of the family's cli-alignment-2026-11-15 plan (round X10): every push to master ran the credentialed live suites, which burns the providers' quotas (Google's calendar quota, Graph throttling) and collides with the owner's live testing. The family moves its live jobs off push, each repository on its own weekly slot so no two suites run together.

## What

- .github/workflows/tests.yml: `workflow_dispatch` and a weekly `schedule` (`0 1 * * 1`, Himalaya's slot) join `push`; the `live` job runs only on those two events, its matrix, secrets and steps unchanged; the offline `tests` and `features` jobs stay on every push.
- CONTRIBUTING.md says when the live suites run, how to run them by hand (`gh workflow run tests.yml`), and that a release runs them on the commit it tags.
- Himalaya's live suites create no Google calendar (mail only), so the long-lived calendar rule of the brief does not apply here.
