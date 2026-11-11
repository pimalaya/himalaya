---
cairn: tasks
change: first-run-wizard-frame
---

# Tasks

- [x] Add a `configure` command (alias `wizard`) running the wizard by name, with no welcome and a non-interactive bail.
- [x] Move the welcome onto the offer, and give it the configuration path that was looked for, the sample URL and the `himalaya configure` pointer.
- [x] Add `offer_configuration`, raised by a bare `himalaya` and by account resolution, returning whether the wizard ran.
- [x] Make account resolution a hook: drop the `exit(0)`, re-read the configuration afterwards, and let the command fail the ordinary way when nothing landed.
- [x] Guard both entry points on `stdin().is_terminal()` and `printer.is_json()`.
- [x] Resolve the target path from `Config::target_path` instead of prompting for it.
- [x] Append to an existing configuration as plain text; suffix the account name until free; claim `default` only when no other account does.
- [x] Name all three resolution failures: the path read, the accounts held, the two ways to pick a default.
- [x] Report where the account landed, under which name, and what to run next.
- [x] Tests: a generated account parses back, an appended one keeps the existing account and its comments, a taken name gets a suffix, a missing configuration constrains nothing.
- [x] Build/test/fmt/clippy.
- [x] Fold into cairn/spec/wizard.md; log; land.
