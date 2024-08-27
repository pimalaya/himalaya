<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI to manage emails, based on <a href="https://crates.io/crates/email-lib"><code>email-lib</code></a></p>
  <p>
    <a href="https://github.com/pimalaya/himalaya/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/pimalaya/himalaya?color=success"/></a>
	<a href="https://repology.org/project/himalaya/versions"><img alt="Repology" src="https://img.shields.io/repology/repositories/himalaya"></a>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/matrix/pimalaya:matrix.org?color=success&label=chat"/></a>
  </p>
</div>

```
$ himalaya envelope list --account posteo --folder Archives.FOSS --page 2
```

![screenshot](./screenshot.jpeg)

## Features

- Interactive configuration via **wizard** (requires `wizard` feature)
- Mailbox/folder management (**create**, **list**, **expunge**, **purge**, **delete**)
- Envelope **listing**, **filtering** and **sorting**
- Message composition based on `$EDITOR`
- Message manipulation (**copy**, **move**, **delete**)
- Multi-accounting
- **JSON** output with `--output json`
- Basic backends:
  - **IMAP** (requires `imap` feature)
  - **Maildir** (requires `maildir` feature)
  - **Notmuch** (requires `notmuch` feature)
- Sending backends:
  - **SMTP** (requires `smtp` feature)
  - **Sendmail** (requires `sendmail` feature)
- PGP encryption:
  - via shell commands (requires `pgp-commands` feature)
  - via [GPG](https://www.gnupg.org/) bindings (requires `pgp-gpg` feature)
  - via native implementation (requires `pgp-native` feature)
- Global system **keyring** for managing secrets (requires `keyring` feature)
- **OAuth 2.0** authorization (requires `oauth2` feature)

*Himalaya CLI is written in [Rust](https://www.rust-lang.org/), and relies on [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) to enable or disable functionalities.*

*Default features can be found in the `features` section of the [`Cargo.toml`](https://github.com/pimalaya/himalaya/blob/master/Cargo.toml#L18).*

## Installation [![Repology](https://img.shields.io/repology/repositories/himalaya)](https://repology.org/project/himalaya/versions)

<details>
  <summary>Prebuilt binary</summary>

  Himalaya CLI can be installed with a prebuilt binary:

  ```bash
  # As root:
  $ curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh
  
  # As a regular user:
  $ curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
  ```

  These commands install the latest binary from the GitHub [releases](https://github.com/pimalaya/himalaya/releases) section.

  *Binaries are built with default cargo features. If you want to enable or disable a feature, please use another installation method.*
</details>

<details>
  <summary>Cargo</summary>

  Himalaya CLI can be installed with [cargo](https://doc.rust-lang.org/cargo/):

  ```bash
  $ cargo install himalaya
  
  # With only IMAP support:
  $ cargo install himalaya --no-default-features --features imap
  ```

  You can also use the git repository for a more up-to-date (but less stable) version:

  ```bash
  $ cargo install --git https://github.com/pimalaya/himalaya.git himalaya
  ```
</details>

<details>
  <summary>Arch Linux</summary>

  Himalaya CLI can be installed on [Arch Linux](https://archlinux.org/) with either the community repository:

  ```bash
  $ pacman -S himalaya
  ```

  or the [user repository](https://aur.archlinux.org/):

  ```bash
  $ git clone https://aur.archlinux.org/himalaya-git.git
  $ cd himalaya-git
  $ makepkg -isc
  ```

  If you use [yay](https://github.com/Jguer/yay), it is even simplier:

  ```bash
  $ yay -S himalaya-git
  ```

</details>

<details>
  <summary>Homebrew</summary>

  Himalaya CLI can be installed with [Homebrew](https://brew.sh/):

  ```bash
  $ brew install himalaya
  ```

</details>

<details>
  <summary>Scoop</summary>

  Himalaya CLI can be installed with [Scoop](https://scoop.sh/):

  ```bash
  $ scoop install himalaya
  ```

</details>

<details>
  <summary>Fedora Linux/CentOS/RHEL</summary>

  Himalaya CLI can be installed on [Fedora Linux](https://fedoraproject.org/)/CentOS/RHEL via [COPR](https://copr.fedorainfracloud.org/coprs/atim/himalaya/) repo:

  ```bash
  $ dnf copr enable atim/himalaya
  $ dnf install himalaya
  ```

</details>

<details>
  <summary>Nix</summary>

  Himalaya CLI can be installed with [Nix](https://serokell.io/blog/what-is-nix):

  ```bash
  $ nix-env -i himalaya
  ```

  You can also use the git repository for a more up-to-date (but less stable) version:

  ```bash
  $ nix-env -if https://github.com/pimalaya/himalaya/archive/master.tar.gz
  
  # or, from within the source tree checkout
  $ nix-env -if .
  ```

  If you have the [Flakes](https://nixos.wiki/wiki/Flakes) feature enabled:

  ```bash
  $ nix profile install himalaya
  
  # or, from within the source tree checkout
  $ nix profile install
  
  # you can also run Himalaya directly without installing it:
  $ nix run himalaya
  ```
</details>

<details>
  <summary>Sources</summary>

  Himalaya CLI can be installed from sources.

  First you need to install the Rust development environment (see the [rust installation documentation](https://doc.rust-lang.org/cargo/getting-started/installation.html)):

  ```bash
  $ curl https://sh.rustup.rs -sSf | sh
  ```

  Then, you need to clone the repository and install dependencies:

  ```bash
  $ git clone https://github.com/pimalaya/himalaya.git
  $ cd himalaya
  $ cargo check
  ```

  Now, you can build Himalaya:

  ```bash
  $ cargo build --release
  ```

  *Binaries are available under the `target/release` folder.*
</details>

## Configuration

Just run `himalaya`, the wizard will help you to configure your default account.

You can also manually write your own configuration, from scratch:

- Copy the content of the documented [`./config.sample.toml`](./config.sample.toml)
- Paste it in a new file `~/.config/himalaya/config.toml`
- Edit, then comment or uncomment the options you want

## FAQ

<details>
  <summary>How to debug Himalaya CLI?</summary>

  The simplest way is to use `--debug` and `--trace` arguments.

  The advanced way is based on environment variables:

  - `RUST_LOG=<level>`: determines the log level filter, can be one of `off`, `error`, `warn`, `info`, `debug` and `trace`.
  - `RUST_SPANTRACE=1`: enables the spantrace (a span represent periods of time in which a program was executing in a particular context).
  - `RUST_BACKTRACE=1`: enables the error backtrace.
  - `RUST_BACKTRACE=full`: enables the full error backtrace, which include source lines where the error originated from.

  Logs are written to the `stderr`, which means that you can redirect them easily to a file:

  ```
  RUST_LOG=debug himalaya 2>/tmp/himalaya.log
  ```
</details>

<details>
  <summary>How the wizard discovers IMAP configs?</summary>

  All the lookup mechanisms use the email address domain as base for the lookup. It is heavily inspired from the Thunderbird [Autoconfiguration](https://udn.realityripple.com/docs/Mozilla/Thunderbird/Autoconfiguration) protocol. For example, for the email address `test@example.com`, the lookup is performed as (in this order):

  1. check for `autoconfig.example.com`
  2. look up of `example.com` in the ISPDB (the Thunderbird central database)
  3. look up `MX example.com` in DNS, and for `mx1.mail.hoster.com`, look up `hoster.com` in the ISPDB
  4. look up `SRV example.com` in DNS
  5. try to guess (`imap.example.com`, `smtp.example.com`…)
</details>

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/project/Himalaya/index.html)

Special thanks to the [NLnet foundation](https://nlnet.nl/project/Himalaya/index.html) and the [European Commission](https://www.ngi.eu/) that helped the project to receive financial support from:

- [NGI Assure](https://nlnet.nl/assure/) in 2022
- [NGI Zero Entrust](https://nlnet.nl/entrust/) in 2023

If you appreciate the project, feel free to donate using one of the following providers:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
