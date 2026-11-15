---
cairn: delta
change: the-live-suites-run-weekly
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Live provider suites
The binary SHALL carry one ignored integration suite per provider under tests/ (google.rs, microsoft.rs, fastmail.rs), run with `--ignored` and sharing tests/common.rs. Each SHALL hold an `anonymous` module that runs every backend the provider's sample offers on a made-up token and touches no account, its accounts taken from the provider sections of config.sample.toml where one fits so a provider drifting from the sample fails it, the accounts it writes checked to load offline by a test that is not ignored, and asserting the exit code of the output contract (3 with the `account check` report, 1 with the failure object) and a message naming the refusal in Himalaya's words. It MAY hold a `credentialed` module that mints its own token from `HIMALAYA_`-prefixed variables and only reads (`account check`, `mailbox list`, `envelope list`), failing naming the variables it misses, which CI runs in the upstream repository only and once its secrets exist. A token SHALL reach the binary through a token command reading its environment, never argv nor the configuration, and no assertion SHALL print a token or secret. CI SHALL run each suite in its own job at `RUST_LOG=debug` at most, weekly (Mondays at 01:00 UTC, a slot of its own among the family's suites) and by hand (`workflow_dispatch`, `gh workflow run tests.yml`), never on a push, the offline jobs alone running on every push; a release SHALL run them by hand on the commit it tags.

## REMOVED Requirements
