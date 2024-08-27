<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI to manage emails, based on <a href="https://crates.io/crates/email-lib"><code>email-lib</code></a></p>
  <p>
    <a href="https://github.com/pimalaya/himalaya/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/pimalaya/himalaya?color=success"/></a>
	<a href="https://repology.org/project/himalaya/versions"><img alt="Repology" src="https://img.shields.io/repology/repositories/himalaya?color=success"></a>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/matrix/pimalaya:matrix.org?color=success&label=chat"/></a>
  </p>
</div>

```
$ himalaya envelope list --account posteo --folder Archives.FOSS --page 2
```

![screenshot](./screenshot.jpeg)

## Features

- Multi-accounting
- Interactive configuration via **wizard** (requires `wizard` feature)
- Mailbox/folder management (**create**, **list**, **expunge**, **purge**, **delete**)
- Envelope **listing**, **filtering** and **sorting**
- Message composition based on `$EDITOR`
- Message manipulation (**copy**, **move**, **delete**)
- Basic backends:
  - **IMAP** (requires `imap` feature)
  - **Maildir** (requires `maildir` feature)
  - **Notmuch** (requires `notmuch` feature)
- Default backends:
  - **SMTP** (requires `smtp` feature)
  - **Sendmail** (requires `sendmail` feature)
- PGP encryption:
  - via shell commands (requires `pgp-commands` feature)
  - via [GPG](https://www.gnupg.org/) bindings (requires `pgp-gpg` feature)
  - via native implementation (requires `pgp-native` feature)
- Global system **keyring** for managing secrets (requires `keyring` feature)
- **OAuth 2.0** authorization (requires `oauth2` feature)
- **JSON** output via `--output json`

*Himalaya CLI is written in [Rust](https://www.rust-lang.org/), and relies on [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) to enable or disable functionalities.*

*Default features can be found in the `features` section of the [`Cargo.toml`](https://github.com/pimalaya/himalaya/blob/master/Cargo.toml#L18).*

## Installation

Himalaya CLI can be installed with a prebuilt binary:

```bash
# As root:
$ curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh

# As a regular user:
$ curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
```

These commands install the latest binary from the GitHub [releases](https://github.com/pimalaya/himalaya/releases) section.

*Binaries are built with [default](https://github.com/pimalaya/himalaya/blob/master/Cargo.toml#L18) cargo features. If you want to enable or disable a feature, please use another installation method.*

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

<details>
  <summary>Proton Mail (Bridge)</summary>

  When using Proton Bridge, emails are synchronized locally and exposed via a local IMAP/SMTP server. This implies 2 things:

  - Id order may be reversed or shuffled, but envelopes will still be sorted by date.
  - SSL/TLS needs to be deactivated manually.
  - The password to use is the one generated by Proton Bridge, not the one from your Proton Mail account.

  ```toml
  [accounts.proton]
  email = "example@proton.me"

  backend = "imap"
  imap.host = "127.0.0.1"
  imap.port = 1143
  imap.encryption = false
  imap.login = "example@proton.me"
  imap.passwd.raw = "<bridge-imap-p@ssw0rd>"

  message.send.backend = "smtp"
  smtp.host = "127.0.0.1"
  smtp.port = 1025
  smtp.encryption = false
  smtp.login = "example@proton.me"
  smtp.passwd.raw = "<bridge-smtp-p@ssw0rd>"
  ```

  Keeping your password inside the configuration file is good for testing purpose, but it is not safe. You have 2 better alternatives:

  - Save your password in any password manager that can be queried via the CLI:

    ```toml
    imap.passwd.cmd = "pass show proton"
    ```

  - Use the global keyring of your system (requires the `keyring` cargo feature):

    ```toml
    imap.passwd.keyring = "proton-example"
    ```

    Running `himalaya configure -a proton` will ask for your IMAP password, just paste the one generated previously.
</details>

<details>
  <summary>Gmail</summary>

  Google passwords cannot be used directly. There is two ways to authenticate yourself:

  ## Using [App Passwords](https://support.google.com/mail/answer/185833)

  This option is the simplest and the fastest. First, be sure that:

  - IMAP is enabled
  - Two-step authentication is enabled
  - Less secure app access is enabled

  First create a [dedicated password](https://myaccount.google.com/apppasswords) for Himalaya.

  ```toml
  [accounts.gmail]
  email = "example@gmail.com"

  folder.alias.inbox = "INBOX"
  folder.alias.sent = "[Gmail]/Sent Mail"
  folder.alias.drafts = "[Gmail]/Drafts"
  folder.alias.trash = "[Gmail]/Trash"

  backend = "imap"
  imap.host = "imap.gmail.com"
  imap.port = 993
  imap.login = "example@gmail.com"
  imap.passwd.cmd = "pass show gmail"

  message.send.backend = "smtp"
  smtp.host = "smtp.gmail.com"
  smtp.port = 465
  smtp.login = "example@gmail.com"
  smtp.passwd.cmd = "pass show gmail"
  ```

  Keeping your password inside the configuration file is good for testing purpose, but it is not safe. You have 2 better alternatives:

  - Save your password in any password manager that can be queried via the CLI:

    ```toml
    imap.passwd.cmd = "pass show gmail"
    ```

  - Use the global keyring of your system (requires the `keyring` cargo feature):

    ```toml
    imap.passwd.keyring = "gmail-example"
    ```

    Running `himalaya configure -a gmail` will ask for your IMAP password, just paste the one generated previously.

  ## Using OAuth 2.0

  This option is the most secure but the hardest to configure. It requires the `oauth2` and `keyring` cargo features.

  First, you need to get your OAuth 2.0 credentials by following [this guide](https://developers.google.com/identity/protocols/oauth2#1.-obtain-oauth-2.0-credentials-from-the-dynamic_data.setvar.console_name-.). Once you get your client id and your client secret, you can configure your Himalaya account this way:

  ```toml
  [accounts.gmail]
  email = "example@gmail.com"

  folder.alias.inbox = "INBOX"
  folder.alias.sent = "[Gmail]/Sent Mail"
  folder.alias.drafts = "[Gmail]/Drafts"
  folder.alias.trash = "[Gmail]/Trash"

  backend = "imap"
  imap.host = "imap.gmail.com"
  imap.port = 993
  imap.login = "example@gmail.com"
  imap.oauth2.client-id = "<imap-client-id>"
  imap.oauth2.auth-url = "https://accounts.google.com/o/oauth2/v2/auth"
  imap.oauth2.token-url = "https://www.googleapis.com/oauth2/v3/token"
  imap.oauth2.pkce = true
  imap.oauth2.scope = "https://mail.google.com/"

  message.send.backend = "smtp"
  smtp.host = "smtp.gmail.com"
  smtp.port = 465
  smtp.login = "example@gmail.com"
  smtp.oauth2.client-id = "<smtp-client-id>"
  smtp.oauth2.auth-url = "https://accounts.google.com/o/oauth2/v2/auth"
  smtp.oauth2.token-url = "https://www.googleapis.com/oauth2/v3/token"
  smtp.oauth2.pkce = true
  smtp.oauth2.scope = "https://mail.google.com/"

  # If you want your SMTP to share the same client id (and so the same access token)
  # as your IMAP config, you can add the following:
  #
  # imap.oauth2.client-id = "<client-id>"
  # imap.oauth2.client-secret.keyring = "gmail-oauth2-client-secret"
  # imap.oauth2.access-token.keyring = "gmail-oauth2-access-token"
  # imap.oauth2.refresh-token.keyring = "gmail-oauth2-refresh-token"
  #
  # imap.oauth2.client-id = "<client-id>"
  # imap.oauth2.client-secret.keyring = "gmail-oauth2-client-secret"
  # imap.oauth2.access-token.keyring = "gmail-oauth2-access-token"
  # smtp.oauth2.refresh-token.keyring = "gmail-oauth2-refresh-token"
  ```

  Running `himalaya configure -a gmail` will complete your OAuth 2.0 setup and ask for your client secret.
</details>

<details>
  <summary>Outlook</summary>

    ```toml
  [accounts.outlook]
  email = "example@outlook.com"

  backend = "imap"
  imap.host = "outlook.office365.com"
  imap.port = 993
  imap.login = "example@outlook.com"
  imap.passwd.cmd = "pass show outlook"

  message.send.backend = "smtp"
  smtp.host = "smtp.mail.outlook.com"
  smtp.port = 587
  smtp.encryption = "start-tls"
  smtp.login = "example@outlook.com"
  smtp.passwd.cmd = "pass show outlook"
  ```

  ### Using OAuth 2.0

  This option is the most secure but the hardest to configure. First, you need to get your OAuth 2.0 credentials by following [this guide](https://learn.microsoft.com/en-us/exchange/client-developer/legacy-protocols/how-to-authenticate-an-imap-pop-smtp-application-by-using-oauth). Once you get your client id and your client secret, you can configure your Himalaya account this way:

  ```toml
  [accounts.outlook]
  email = "example@outlook.com"

  backend = "imap"
  imap.host = "outlook.office365.com"
  imap.port = 993
  imap.login = "example@outlook.com"
  imap.oauth2.client-id = "<imap-client-id>"
  imap.oauth2.auth-url = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
  imap.oauth2.token-url = "https://login.microsoftonline.com/common/oauth2/v2.0/token"
  imap.oauth2.pkce = true
  imap.oauth2.scope = "https://outlook.office.com/IMAP.AccessAsUser.All"

  message.send.backend = "smtp"
  smtp.host = "smtp.mail.outlook.com"
  smtp.port = 587
  smtp.starttls = true
  smtp.login = "example@outlook.com"
  smtp.oauth2.client-id = "<smtp-client-id>"
  smtp.oauth2.auth-url = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
  smtp.oauth2.token-url = "https://login.microsoftonline.com/common/oauth2/v2.0/token"
  smtp.oauth2.pkce = true
  smtp.oauth2.scope = "https://outlook.office.com/SMTP.Send"

  # If you want your SMTP to share the same client id (and so the same access token)
  # as your IMAP config, you can add the following:
  #
  # imap.oauth2.client-id = "<client-id>"
  # imap.oauth2.client-secret.keyring = "outlook-oauth2-client-secret"
  # imap.oauth2.access-token.keyring = "outlook-oauth2-access-token"
  # imap.oauth2.refresh-token.keyring = "outlook-oauth2-refresh-token"
  #
  # imap.oauth2.client-id = "<client-id>"
  # imap.oauth2.client-secret.keyring = "outlook-oauth2-client-secret"
  # imap.oauth2.access-token.keyring = "outlook-oauth2-access-token"
  # smtp.oauth2.refresh-token.keyring = "outlook-oauth2-refresh-token"
  ```

  Running `himalaya configure -a outlook` will complete your OAuth 2.0 setup and ask for your client secret.
</details>

<details>
  <summary>iCloud Mail</summary>

  From the [iCloud Mail](https://support.apple.com/en-us/HT202304) support page:

  - IMAP port = `993`.
  - IMAP login = name of your iCloud Mail email address (for example, `johnappleseed`, not `johnappleseed@icloud.com`)
  - SMTP port = `587` with `STARTTLS`
  - SMTP login = full iCloud Mail email address (for example, `johnappleseed@icloud.com`, not `johnappleseed`)

  ```toml
  [accounts.icloud]
  email = "johnappleseed@icloud.com"

  backend = "imap"
  imap.host = "imap.mail.me.com"
  imap.port = 993
  imap.login = "johnappleseed"
  imap.passwd.cmd = "pass show icloud"

  message.send.backend = "smtp"
  smtp.host = "smtp.mail.me.com"
  smtp.port = 587
  smtp.encryption = "start-tls"
  smtp.login = "johnappleseed@icloud.com"
  smtp.passwd.cmd = "pass show icloud"
  ```
</details>

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
  <summary>How the wizard discovers IMAP/SMTP configs?</summary>

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
