---
cairn: tasks
change: secret-command-resolved-once
---

# Tasks

- [x] Move `pimalaya-config` from 0.1 to 0.2.
- [x] Build `CommandConfig::Argv` and `CommandConfig::Shell` in the wizard secret module, converging on the neverest reference.
- [x] Take a `&mut SecretResolver` in `SaslConfig::try_into_sasl`, `jmap_http_auth`, `gmail_token` and `msgraph_token`.
- [x] Build one resolver in `account check` and in `test_account`, threaded through every `connect_*`.
- [x] Share one resolver between the wizard's IMAP and SMTP connection tests.
- [x] Add `SieveClient::new_with`, so the account checker reaches ManageSieve through its resolver.
- [x] Check the feature matrix, the secret paths being gated per backend.
- [x] Changelog entry.
