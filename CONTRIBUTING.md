# Contributing guide

Thank you for investing your time in contributing to Himalaya.

Whether you are a human or an AI agent, read these in order before touching the code:

1. the [Pimalaya README](https://github.com/pimalaya) for what the project is and how its repositories stack;
2. the [Pimalaya CONTRIBUTING](https://github.com/pimalaya/.github/blob/master/CONTRIBUTING.md) guide, which chains to the shared architecture and guidelines;
3. the inline header documentation, starting with src/lib.rs: it is the architecture document of this crate;
4. the cairn/ folder, which follows [Cairn](https://github.com/pimalaya/cairn): spec/ is current truth, changes/ holds in-flight proposals (landed ones move to changes/archive/), and log/ is the dated history; `sh cairn/verify.sh` checks the structure.

Everything below documents only what differs from the Pimalaya standards.

## Where changes belong

Himalaya is the CLI front-end of the Pimalaya email stack, a thin shell driving the sans-I/O io- libraries. Triage before patching, since protocol and storage fixes usually belong upstream:

- IMAP, JMAP, Gmail, Microsoft Graph and SMTP wire semantics belong in [io-imap](https://github.com/pimalaya/io-imap), [io-jmap](https://github.com/pimalaya/io-jmap), [io-gmail](https://github.com/pimalaya/io-gmail), [io-msgraph](https://github.com/pimalaya/io-msgraph) and [io-smtp](https://github.com/pimalaya/io-smtp);
- local storage semantics belong in [io-maildir](https://github.com/pimalaya/io-maildir) and [io-mbox](https://github.com/pimalaya/io-mbox);
- the commands, rendering, composition and the shared cross-protocol surface live here.

The clap, printer, plain text layout (columns and fields), prompt, first-run welcome, REPL loop and MCP protocol come from [pimalaya/cli](https://github.com/pimalaya/cli), the TOML loader, account resolution and secret resolution from [pimalaya/config](https://github.com/pimalaya/config), the TCP, TLS and SASL plumbing from [pimalaya/stream](https://github.com/pimalaya/stream), and MIME composition from [pimalaya/mml](https://github.com/pimalaya/mml). The src/lib.rs header maps each backend to its crate.

## Feature matrix

Himalaya is a binary, not a layered library, so it has no coroutine/client split. Its cargo features gate the backends (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox` and `pimdir`, all on by default), the ManageSieve service protocol (`sieve`, on by default), the bundled SQLite (`vendored`), and the TLS provider (`rustls-ring` default, `rustls-aws`, `native-tls`). Build a reduced set to check the feature gates still hold when touching them:

```sh
cargo build --no-default-features --features imap,smtp,rustls-ring
cargo build --no-default-features --features jmap,rustls-ring
cargo test --no-default-features --features sieve,rustls-ring
```

## Dependencies

Cargo.toml patches the Pimalaya crates still ahead of their crates.io release to git. To build against a local checkout, swap the matching `.git` entry for `.path = "../<repo>"`. If cargo reports two versions of a crate, patch every Pimalaya crate that pulls it transitively so the graph converges on the local copies.

## Continuous integration

Every push runs the offline jobs of .github/workflows/tests.yml: the tests, and clippy over every feature set a packager may pick (the `features` job). The live suites (tests/google.rs, tests/microsoft.rs, tests/fastmail.rs, the `live` job), which reach the real providers and spend their quotas, run weekly on Mondays at 01:00 UTC and by hand, never on a push:

```sh
gh workflow run tests.yml
```

A release runs them by hand on the commit it tags, and tags it once they pass.
