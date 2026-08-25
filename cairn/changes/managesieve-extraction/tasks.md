---
cairn: change-tasks
id: managesieve-extraction
status: landed
---

- [x] Create io-managesieve with the I/O-free coroutines, the session opener and the std client.
- [x] Replace src/sieve/protocol.rs and src/sieve/client.rs with a wrapper over the library.
- [x] Move the fake-server coverage into io-managesieve's integration tests.
- [x] Widen the Sieve SASL surface to every mechanism the other backends accept.
- [x] Add `sieve rename` and the `sieve.allow-cleartext-auth` config field.
- [x] Update the README, config.sample.toml and CHANGELOG.
- [x] Run fmt, clippy, the feature matrix and the test suite on both repositories.
- [x] Fold the change into current Cairn specs and write a landing log.
