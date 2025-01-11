# Contributing guide

Thank you for investing your time in contributing to Himalaya CLI!

## Development

The development environment is managed by [Nix](https://nixos.org/download.html).
Running `nix-shell` will spawn a shell with everything you need to get started with the lib.

If you do not want to use Nix, you can either use [rustup](https://rust-lang.github.io/rustup/index.html):

```text
rustup update
```

or install manually the following dependencies:

- [cargo](https://doc.rust-lang.org/cargo/) (`v1.82`)
- [rustc](https://doc.rust-lang.org/stable/rustc/platform-support.html) (`v1.82`)

## Build

```text
cargo build
```

You can disable default [features](https://doc.rust-lang.org/cargo/reference/features.html) with `--no-default-features` and enable features with `--features feat1,feat2,feat3`.

Finally, you can build a release with `--release`:

```text
cargo build --no-default-features --features imap,smtp,keyring --release
```

## Override dependencies

If you want to build Himalaya CLI with a dependency installed locally (for example `email-lib`), then you can [override it](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html) by adding the following lines at the end of `Cargo.toml`:

```toml
[patch.crates-io]
email-lib = { path = "/path/to/email-lib" }
```

If you get the following error:

```text
note: perhaps two different versions of crate email are being used?
```

then you may need to override more Pimalaya's sub-dependencies:

```toml
[patch.crates-io]
email-lib.path = "/path/to/core/email"
imap-client.path = "/path/to/imap-client"
keyring-lib.path = "/path/to/core/keyring"
mml-lib.path = "/path/to/core/mml"
oauth-lib.path = "/path/to/core/oauth"
pgp-lib.path = "/path/to/core/pgp"
pimalaya-tui.path = "/path/to/tui"
process-lib.path = "/path/to/core/process"
secret-lib.path = "/path/to/core/secret"
```

*See [pimalaya/core#32](https://github.com/pimalaya/core/issues/32) for more information.*

## Commit style

Starting from the `v1.0.0`, Himalaya CLI tries to adopt the [conventional commits specification](https://github.com/conventional-commits/conventionalcommits.org).
