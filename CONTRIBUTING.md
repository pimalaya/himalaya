# Contributing guide

Thank you for investing your time in contributing to Himalaya CLI!

## Development

The development environment is managed by [Nix](https://nixos.org/download.html). Running `nix-shell` will spawn a shell with everything you need to get started with the lib: `cargo`, `cargo-watch`, `rust-bin`, `rust-analyzer`, `notmuch`…

```sh
# Start a Nix shell
$ nix-shell

# then build the CLI
$ cargo build

# run the CLI
$ cargo run --feature pgp-gpg -- envelope list
```

## Commit style

Starting from the `v1.0.0`, Himalaya CLI tries to adopt the [conventional commits specification](https://github.com/conventional-commits/conventionalcommits.org).
