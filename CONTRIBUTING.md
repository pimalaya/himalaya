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

## Contributing

Himalaya CLI supports open-source, hence the choice of using [SourceHut](https://sourcehut.org/) for managing the project. The only reason why the source code is hosted on GitHub is to build releases for all major platforms (using GitHub Actions). Don't worry, contributing on SourceHut is not a big deal: you just need to send emails! You don't need to create any account. Here a small comparison guide with GitHub:

The equivalent of **GitHub Discussions** are:

- The [Matrix](https://matrix.org/) chat room [#pimalaya.himalaya](https://matrix.to/#/#pimalaya.himalaya:matrix.org)
- The SourceHut mailing list. You can consult existing messages [here](https://lists.sr.ht/~soywod/pimalaya). You can "open a new discussion" by sending an email at [~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht). You can also [subscribe](mailto:~soywod/pimalaya+subscribe@lists.sr.ht) and [unsubscribe](mailto:~soywod/pimalaya+unsubscribe@lists.sr.ht) to the mailing list, so you can receive a copy of all discussions.

The equivalent of **GitHub Issues** is the SourceHut bug tracker. You can consult existing bugs [here](https://todo.sr.ht/~soywod/pimalaya), and you can "open a new issue" by sending an email at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

The equivalent of **GitHub Pull requests** is the SourceHut mailing list. You can "open a new pull request" by sending an email containing a git patch at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht). The simplest way to send a patch is to use [git send-email](https://git-scm.com/docs/git-send-email), follow [this guide](https://git-send-email.io/) to configure git properly.
