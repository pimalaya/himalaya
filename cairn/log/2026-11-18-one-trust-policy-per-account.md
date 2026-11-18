---
cairn: log
change: one-trust-policy-per-account
landed: 2026-11-18
---

# One TLS trust policy per account

The draft of 2026-10-18, implemented as proposed. Its open questions are settled as follows: the per-transport `tls` stays, overriding field by field; under native-tls the platform store stays the trust and `cert` adds to it, as before.

## What landed

- src/config.rs, src/config/tls.rs: `AccountConfig::tls`, inherited field by field by every network backend as the account `proxy` is; unknown keys refused by serde as everywhere.
- src/account/check.rs, src/json_schema.rs: `account check` prints its own report, each row pimalaya-cli's `BackendCheck` beside `tls`; `himalaya-account-check` describes it.
- config.sample.toml: the `tls.*` keys at account level, `imap.tls.cert` as the override.
- Tests `backends_inherit_the_account_tls_field_by_field` and `an_account_check_names_the_trust_of_each_transport`, failing on e440b4e4.
- CHANGELOG (Added).

## Capabilities moved

- [config](../spec/config.md): *One TLS policy per account*, new; *Path keys expand as the configuration is read*.
- [commands](../spec/commands.md): *account check covers every declared backend*.
