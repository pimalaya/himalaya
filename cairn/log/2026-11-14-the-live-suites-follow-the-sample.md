---
cairn: log
change: the-live-suites-follow-the-sample
landed: 2026-11-14
---

# The live suites follow the sample

The sample rework of the same day (the-sample-lists-values-in-brackets) broke the three live suites in CI. tests/common.rs `sample_section` skipped the line after a heading, a blank comment line before, and uncommented every line, so the Gmail and Graph sections now set a token's `command` and `raw` together, which the configuration refuses. The Fastmail and Microsoft IMAP and SMTP sections were gone.

`sample_section` now takes the paragraph a heading opens, keeping the first line of each field (a secret's `raw` after its `command` left out). The Gmail and Graph suites still read their sections. Fastmail and Exchange IMAP and SMTP, which the sample no longer presets, write their accounts in the test. Each suite gained a test that is not ignored, loading every account it writes offline through `account list`. It failed against the old helper with the CI's own error.

Spec: packaging (the live suites' accounts are checked to load offline).
