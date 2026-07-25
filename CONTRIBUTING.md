# Contributing guide

Thank you for investing your time in contributing to Himalaya CLI.

Whether you are a human or an AI agent, read these in order before touching the code:

1. the [Pimalaya README](https://github.com/pimalaya) for what the project is and how its repositories stack;
2. the [Pimalaya CONTRIBUTING](https://github.com/pimalaya/.github/blob/master/CONTRIBUTING.md) guide (Nix environment, build and check commands, dependency overrides, commit style), which chains to the shared architecture and guidelines;
3. the inline header documentation in src/main.rs: it is the architecture document of this binary, covering the backends and plumbing, the three command families, and the shared versus protocol-specific surface;
4. the cairn/ folder, which follows [Cairn](https://github.com/pimalaya/cairn) (spec/ is current truth, changes/ holds in-flight proposals, log/ is the dated history), activated by AGENTS.md.

Everything below documents only what differs from the Pimalaya standards.

## Where changes belong

Himalaya is the CLI front-end of the Pimalaya email stack, a thin shell driving the sans-I/O io- libraries. Triage before patching, since protocol and storage fixes usually belong upstream:

- IMAP, JMAP, Gmail, Microsoft Graph and SMTP wire semantics belong in [io-imap](https://github.com/pimalaya/io-imap), [io-jmap](https://github.com/pimalaya/io-jmap), [io-gmail](https://github.com/pimalaya/io-gmail), [io-msgraph](https://github.com/pimalaya/io-msgraph) and [io-smtp](https://github.com/pimalaya/io-smtp);
- local storage semantics belong in [io-maildir](https://github.com/pimalaya/io-maildir) and [io-m2dir](https://github.com/pimalaya/io-m2dir);
- account discovery consumed by the wizard belongs in [io-pim-discovery](https://github.com/pimalaya/io-pim-discovery);
- the commands, rendering, composition, the wizard and the shared cross-protocol surface live here.

The clap, printer, prompt and spinner primitives come from [pimalaya/cli](https://github.com/pimalaya/cli), the TOML loader and secret resolution from [pimalaya/config](https://github.com/pimalaya/config), the TCP, TLS and SASL plumbing from [pimalaya/stream](https://github.com/pimalaya/stream), and MIME composition from [pimalaya/mml](https://github.com/pimalaya/mml). The src/main.rs header maps each backend to its crate.

## Feature matrix

Himalaya is a binary, not a layered library, so it has no coroutine/client split. Its cargo features gate the backends (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `m2dir`), the setup `wizard`, and the TLS provider (`rustls-ring` default, `rustls-aws`, `native-tls`), all on by default. Build a reduced set to check the feature gates still hold when touching them:

```sh
cargo build --no-default-features --features imap,smtp,rustls-ring
cargo build --no-default-features --features jmap,rustls-ring
```

## Dependencies

Cargo.toml patches the Pimalaya crates still ahead of their crates.io release to git. To build against a local checkout, swap the matching `.git` entry for `.path = "../<repo>"`. If cargo reports two versions of a crate, patch every Pimalaya crate that pulls it transitively so the graph converges on the local copies.
