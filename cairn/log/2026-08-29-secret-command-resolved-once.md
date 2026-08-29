---
cairn: log
change: secret-command-resolved-once
landed: 2026-08-29
---

# A credential command is resolved once per run

himalaya moved to pimalaya-config 0.2.0 and adopted its `SecretResolver`, so a credential command several blocks of one account name is spawned once per run instead of once per block.

The waste was real and easy to hit. `[imap]`, `[smtp]` and `[sieve]` each hold their own `password.command`, the natural configuration points all three at one entry, and `Secret::get` caches nothing: `account check` spawned it three times, and the wizard's IMAP and SMTP tests twice even after answering yes to "Use the same credentials for SMTP?". For a `pass` or `gpg` entry each spawn is a key unlock, so validating an account meant unlocking once per backend.

What made the memo possible is the type change under it: `Secret::Command` now carries a `pimalaya_config::command::CommandConfig`, a shell line or a program with its arguments, instead of a built `std::process::Command`. That is comparable and cheap to clone where a `Command` is neither, and comparing it never crosses the two shapes. Both TOML spellings parse and serialize exactly as before, so no configuration file changed.

The wizard secret module converged on the neverest reference: `command_secret` builds `CommandConfig::Argv` and `shell_secret` builds `CommandConfig::Shell`, the empty argv and the blank shell line are still refused, and the two validation tests stand.

`SaslConfig::try_into_sasl`, `jmap_http_auth`, `gmail_token` and `msgraph_token` now resolve through a `&mut SecretResolver` handed to them. Three places build one. `account check` and `test_account` build one per account and thread it through every `connect_*`, which is where three unlocks became one. The discovered-backend wizard builds one for its IMAP and SMTP connection tests, which is where two became one. Everywhere else a client's `new` builds a resolver of its own and drops it with the call, so a command reaching one backend behaves exactly as it did. `SieveClient::new_with` is new, the account checker delegating to the sieve client rather than opening the session itself.

The shared `EmailClient` was deliberately left alone. It connects storage eagerly and SMTP lazily on the first send, so one resolver across the two would have to live on the client for the length of the command, which is what the resolver's contract rules out; resolving SMTP up front instead would spawn the credential command for read-only runs that never send.

Capabilities moved: config gained "A credential command is spawned once per run".

Verified with `cargo fmt`, `cargo check --all-features`, `cargo clippy --all-features` and `cargo test --all-features` (119 tests), all clean, plus a per-feature sweep over every backend flag on its own and in pairs, which added no warning. Two pre-existing faults were seen and left alone: `--features pimdir` alone does not compile, `crate::email::search::eval` being gated on maildir and m2dir, and the wizard secret module reports dead code under feature sets that compile no wizard using it.

Not verified against a live account: the change is a memo in front of a process spawn, and no protocol path moved.
