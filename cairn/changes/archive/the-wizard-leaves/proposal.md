---
cairn: change
id: the-wizard-leaves
status: landed
created: 2026-11-11
---

# The wizard leaves

## Why

The owner decided on 2026-11-08 that the Pimalaya CLIs drop their setup wizards. Himalaya's is broken already: it hands an OAuth 2.0 sign-in to `ortie configure`, which Ortie 3.0 removed, so the path it ranks first for Gmail, Microsoft and JMAP accounts fails. Keeping it means owning discovery (io-pim-discovery), a keyring dialogue, a review UI and a TOML writer in a binary whose job is to drive mail, for a flow the shared welcome of pimalaya-cli replaces family-wide.

## What

- Delete src/wizard/ and the `configure` command (alias `wizard`), with the `wizard` cargo feature, its place in `default` and the io-pim-discovery dependency and its TLS forwards.
- Bare `himalaya` prints the help. A command needing an account and finding no configuration fails with `No configuration found at <path>, write one by hand from the sample: <URL>`, where it used to offer the wizard on a terminal. The shared welcome replaces that message in a later change.
- Drop the `himalaya-configure` JSON Schema, the REPL guard refusing `configure`, the wizard-only parts of `account check` (`test_account`, the unauthenticated SASL probe) and the `Serialize` derives, `skip_serializing_if` attributes and `AccountConfig::render` that only rendered a generated account.
- Switch pimalaya-cli to the local checkout carrying the removal of its own `wizard` module (path patch); shellexpand leaves, the wizard being its only user.
- README, CONTRIBUTING and config.sample.toml stop pointing at the wizard; the six landed wizard changes move to changes/archive/.

## Scope / non-goals

The shared first-run welcome (pimalaya-cli `welcome::Welcome`), the `-c` flag taking one path per flag, the REPL and MCP moves and the pimalaya-config bump are the next change's. `account check` keeps every connector it runs; only what the wizard alone called goes.
