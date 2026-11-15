---
cairn: change
id: secret-command-resolved-once
status: landed
created: 2026-08-29
---

# A credential command is resolved once per run

## Why

An account names its credential once and pays for it several times. `[imap]`, `[smtp]` and `[sieve]` each hold their own `password.command`, and the natural configuration points all three at the same entry. `Secret::get` caches nothing, so `account check` spawned that command three times, and the wizard's IMAP and SMTP connection tests spawned it twice even after answering yes to "Use the same credentials for SMTP?".

For a `pass` or a `gpg` entry each spawn is a key unlock, so validating an account meant typing a passphrase once per backend, or waiting for the agent to prompt again. Nothing about the account changed between the reads.

pimalaya-config 0.2.0 answers this with `SecretResolver`, which spawns each distinct command once and hands the value to every field naming it. Distinctness is `CommandConfig`'s own equality, which never crosses the shell and argv shapes: what the configuration wrote is what is compared, and a resolver keyed on a credential is the last place to guess that two spellings mean one thing.

The same release turns `Secret::Command` from a built `std::process::Command` into that comparable `CommandConfig`, which is what makes the memo possible. Both TOML shapes parse and serialize exactly as before.

## What

himalaya moves to pimalaya-config 0.2.0. The wizard's secret module builds `CommandConfig::Argv` and `CommandConfig::Shell` instead of a `std::process::Command`.

`SaslConfig::try_into_sasl`, `jmap_http_auth`, `gmail_token` and `msgraph_token` take a `&mut SecretResolver` and resolve through it. The two places that reach several backends in one run build one resolver and thread it: `account check` and `test_account` in the account checker, and the IMAP plus SMTP connection tests in the discovered-backend wizard.

A command reaching one backend keeps its own resolver, built inside the client's `new` and dropped with the call, so nothing holds plaintext longer than the connection it opens.

## Scope / non-goals

The shared `EmailClient` is left alone. It connects storage eagerly and SMTP lazily on the first send, so sharing a resolver across the two would mean storing it on the client for the length of the command, which is exactly what the resolver's contract rules out. Resolving SMTP up front instead would spawn the credential command for read-only runs that never send.

No configuration file changes, no new options, and no other dependency moves.
