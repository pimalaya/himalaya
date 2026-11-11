---
cairn: log
change: the-wizard-leaves
landed: 2026-11-11
---

# The wizard leaves

The owner decided on 2026-11-08 that the Pimalaya CLIs drop their wizards; Himalaya's was broken already, handing OAuth 2.0 to `ortie configure`, which Ortie 3.0 removed.

- src/wizard/ (14 files, 4,089 lines) is gone, with the `configure` command and its `wizard` alias, the `wizard` cargo feature and its place in `default`, io-pim-discovery and its forwards in the three TLS features, the pimalaya-cli `wizard`, `imap`, `smtp`, `jmap` and `spinner` features, and shellexpand, the wizard being its only user.
- pimalaya-cli moves to a path patch on ../cli, the checkout removing its own `wizard` module (`[patch.crates-io]`); its `clap::commands` items are imported from their modules (`completion::CompletionCommand`, `manual::ManualCommand`, `json_schema::{JsonSchemaCommand, JsonSchemaRegistry}`).
- Bare `himalaya` prints the help. `resolve_account` no longer takes the printer nor offers anything: no configuration fails with `No configuration found at <path>, write one by hand from the sample: <URL>`, the one `NO_CONFIG_HINT` left in src/config.rs. The `long_about` pitch is gone.
- src/config.rs: `AccountConfig::render`, `RENDER_ORDER`, `CONFIG_SAMPLE_URL`, every `Serialize` derive and every `skip_serializing_if` helper (`is_default`, `is_default_*_alpn`) are gone, nothing else serializing a configuration type: the JSON outputs carry none, and `PostingStyle` is a clap value only. The JMAP, Gmail and Graph configs and their auth types take the `#[allow(unused)]` the other backend configs carry, the serializer having been the only reader of their fields in a build without those backends.
- src/account/check.rs: `test_account` and `probe_imap_mechanisms` were called by the wizard alone and are gone; the connectors are private again. `account check` itself is unchanged.
- src/json_schema.rs drops `himalaya-configure`; src/repl.rs drops the guard refusing `configure` inside a session.
- README (the TIP, the Discovery feature list, the Configuration section, the FAQ entry), CONTRIBUTING and the config.sample.toml header no longer mention the wizard. The CHANGELOG drops the Unreleased wizard entries for one Removed entry.
- The six landed wizard changes (first-run-wizard-frame, wizard-auth-and-mailbox-aliases, wizard-capability-auth-probe, wizard-discovery-only, wizard-manual-smtp-credentials, wizard-private-config) move to changes/archive/.

Verified: `cargo test --all-features` passes (417 unit tests and the integration suites); clippy is clean with `-D warnings` for the default features, `--all-features` and `--no-default-features --features rustls-ring,imap,smtp`. The `json-schema` output of the new binary matches the old one's, structurally (descriptions stripped, jq-sorted), but for the missing `himalaya-configure.json`: 223 schemas with all features.

Spec: wizard removed whole (spec/wizard.md); config (Missing account is an error, One default account, Config never written), commands (The JSON contract is described and versioned), packaging (Feature-gated backends), provider-quirks (Bearer-only proprietary APIs, SASL OAuth carries a username) and repl (A session runs the CLI over one account) modified.

Left for the next change: the shared first-run welcome (pimalaya-cli `welcome::Welcome`) replacing the no-configuration error, `-c` taking one path per flag (`HIMALAYA_CONFIG` and the delimiter go with it), the REPL and MCP moves to pimalaya-cli, the pimalaya-config bump, and the path patch giving way to a released pimalaya-cli.
