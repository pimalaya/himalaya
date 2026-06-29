# Contributing guide

Thank you for investing your time in contributing to Himalaya CLI.

## Development environment

The development environment is managed by [Nix flakes](https://nixos.wiki/wiki/Flakes). Running `nix develop` (or `nix-shell` for non-flake users) spawns a shell with the right Rust toolchain, `cargo-deny`, `pkg-config` and the OpenSSL / DBus libraries.

If you do not want to use Nix, install [rustup](https://rust-lang.github.io/rustup/index.html) and pull the toolchain pinned by `rust-version` in `Cargo.toml`:

```
rustup update
```

- `cargo` (>= `v1.87`)
- `rustc` (>= `v1.87`, edition 2024)

## Build

```
cargo build
```

You can disable default [features](https://doc.rust-lang.org/cargo/reference/features.html) with `--no-default-features` and enable individual features with `--features feat1,feat2`.

For example, an IMAP+SMTP-only release build:

```
cargo build --no-default-features --features imap,smtp,rustls-ring --release
```

The release profile (`[profile.release]` in `Cargo.toml`) sets `lto = "fat"`, `codegen-units = 1`, `strip = "symbols"` and `panic = "abort"` to keep the binary small.

## Project layout

Himalaya CLI is the command-line front-end of the [Pimalaya](https://github.com/pimalaya) project. Most of the work happens in companion crates rather than in this repository:

- [io-email](https://github.com/pimalaya/io-email): cross-protocol email client (`EmailClientStd`, shared `Envelope` / `Mailbox` / `Flag` / `Address` types, search DSL).
- [io-imap](https://github.com/pimalaya/io-imap), [io-jmap](https://github.com/pimalaya/io-jmap), [io-gmail](https://github.com/pimalaya/io-gmail), [io-msgraph](https://github.com/pimalaya/io-msgraph), [io-maildir](https://github.com/pimalaya/io-maildir), [io-m2dir](https://github.com/pimalaya/io-m2dir), [io-smtp](https://github.com/pimalaya/io-smtp): per-protocol I/O-free coroutines plus the std-blocking clients that drive them.
- [io-http](https://github.com/pimalaya/io-http): I/O-free HTTP request/response state machines used by JMAP and the discovery wizard.
- [pimconf](https://github.com/pimalaya/pimconf): PIM service discovery (PACC, Thunderbird Autoconfiguration, RFC 6186 SRV) consumed by the wizard.
- [pimalaya/stream](https://github.com/pimalaya/stream): TCP / TLS / SASL plumbing shared by all std clients.
- [pimalaya/cli](https://github.com/pimalaya/cli): cross-binary CLI helpers (printer, prompt, wizard primitives, clap args, build-time env, spinner).
- [pimalaya/config](https://github.com/pimalaya/config): TOML configuration loader and shell-expanded secrets.
- [pimalaya/mml](https://github.com/pimalaya/mml): MIME Meta Language composer / interpreter, chained into `messages send` / `messages add` via a tempfile or shell process substitution.
- [pimalaya/sirup](https://github.com/pimalaya/sirup): session re-use over a Unix socket (pair with `imap.server` / `smtp.server` to amortize TLS handshakes).
- [pimalaya/ortie](https://github.com/pimalaya/ortie): standalone OAuth 2.0 token broker (replaces v1's bundled `oauth-lib`).

Bugs touching protocol semantics usually live in the matching `io-*` crate; rendering, composition and CLI surface live here.

## Override dependencies

`Cargo.toml` already patches every Pimalaya crate to its git remote so the working copy compiles against the latest `master` of each lib:

```toml
[patch.crates-io]
io-email.git = "https://github.com/pimalaya/io-email"
io-gmail.git = "https://github.com/pimalaya/io-gmail"
io-http.git = "https://github.com/pimalaya/io-http"
io-imap.git = "https://github.com/pimalaya/io-imap"
io-jmap.git = "https://github.com/pimalaya/io-jmap"
io-maildir.git = "https://github.com/pimalaya/io-maildir"
io-msgraph.git = "https://github.com/pimalaya/io-msgraph"
io-smtp.git = "https://github.com/pimalaya/io-smtp"
pimalaya-cli.git = "https://github.com/pimalaya/cli"
pimalaya-config.git = "https://github.com/pimalaya/config"
pimalaya-stream.git = "https://github.com/pimalaya/stream"
pimconf.git = "https://github.com/pimalaya/pimconf"
```

To build against a local checkout of one of those crates, swap the matching `.git = "..."` for `.path = "../<repo>"`. For example, with `io-email` next to `himalaya`:

```toml
[patch.crates-io]
io-email.path = "../io-email"
```

If cargo complains about *"perhaps two different versions of crate X are being used"*, patch every Pimalaya crate that pulls X transitively so the dep graph converges on the local copies:

```toml
[patch.crates-io]
io-email.path = "../io-email"
io-gmail.path = "../io-gmail"
io-http.path = "../io-http"
io-imap.path = "../io-imap"
io-jmap.path = "../io-jmap"
io-maildir.path = "../io-maildir"
io-msgraph.path = "../io-msgraph"
io-smtp.path = "../io-smtp"
pimalaya-cli.path = "../cli"
pimalaya-config.path = "../config"
pimalaya-stream.path = "../stream"
pimconf.path = "../pimconf"
```

## Lint, test, audit

```
cargo fmt
cargo clippy --all-features --all-targets
cargo test --all-features
cargo deny check
```

`cargo deny` runs against the rules in [`deny.toml`](./deny.toml) (license allow-list and allowed git sources).

## Commit style

Himalaya CLI follows the [conventional commits specification](https://www.conventionalcommits.org/en/v1.0.0/#summary). Prefix every commit with one of `feat`, `fix`, `refactor`, `docs`, `chore`, `test`, `ci`, `build`, optionally scoped (`fix(imap): ...`).
