# 📫 Himalaya [![GitHub release](https://img.shields.io/github/v/release/soywod/himalaya?color=success)](https://github.com/soywod/himalaya/releases/latest) [![Matrix](https://img.shields.io/matrix/pimalaya.himalaya:matrix.org?color=success&label=chat)](https://matrix.to/#/#pimalaya.himalaya:matrix.org)

https://pimalaya.org/himalaya/cli/latest/

CLI to manage emails, based on [email-lib](https://crates.io/crates/email-lib).

![screenshot](https://user-images.githubusercontent.com/10437171/138774902-7b9de5a3-93eb-44b0-8cfb-6d2e11e3b1aa.png)

*Disclaimer: the project is under active development, do not use in production before the final `v1.0.0`.*

## Features

- [Mailbox management](https://pimalaya.org/himalaya/cli/latest/usage/folder/)
- [Envelopes listing](https://pimalaya.org/himalaya/cli/latest/usage/envelope/list.md)
- [Message composition](https://pimalaya.org/himalaya/cli/latest/usage/message/write.md) based on `$EDITOR`
- Message manipulation ([copy](https://pimalaya.org/himalaya/cli/latest/usage/message/copy.md)/[move](https://pimalaya.org/himalaya/cli/latest/usage/message/move.md)/[delete](https://pimalaya.org/himalaya/cli/latest/usage/message/delete.md))
- [Multi-accounting](https://pimalaya.org/himalaya/cli/latest/configuration/)
- [Account synchronization](https://pimalaya.org/himalaya/cli/latest/usage/account/sync.md) for offline usage
- Support for [IMAP](https://pimalaya.org/himalaya/cli/latest/configuration/imap.md), [Maildir](https://pimalaya.org/himalaya/cli/latest/configuration/maildir.md), [notmuch](https://pimalaya.org/himalaya/cli/latest/configuration/notmuch.md)
- Sending via [SMTP](https://pimalaya.org/himalaya/cli/latest/configuration/smtp.md) or [sendmail](https://pimalaya.org/himalaya/cli/latest/configuration/sendmail.md)
- [PGP](https://pimalaya.org/himalaya/cli/latest/configuration/pgp/) end-to-end encryption
- Generate [completion scripts](https://pimalaya.org/himalaya/cli/latest/tips/completion.md) for various shells
- Generate [man pages](https://pimalaya.org/himalaya/cli/latest/tips/man.md)
- JSON output
- …and more!

## Installation

<table align="center">
<tr>
<td width="50%">
<a href="https://repology.org/project/himalaya/versions">
<img src="https://repology.org/badge/vertical-allrepos/himalaya.svg" alt="Packaging status" />
</a>
</td>
<td width="50%">

```bash
# Arch Linux (official)
$ pacman -S himalaya

# Arch Linux (from sources)
$ yay -S himalaya-git

# Homebrew
$ brew install himalaya

# Scoop
$ scoop install himalaya

# Cargo
$ cargo install himalaya

# Nix
$ nix-env -i himalaya

# Fedora/CentOS
$ dnf copr enable atim/himalaya
$ dnf install himalaya
```

*See the [documentation](https://pimalaya.org/himalaya/cli/latest/installation/) for other installation methods.*

</td>
</tr>
</table>

## Configuration

Please read the [documentation](https://pimalaya.org/himalaya/cli/latest/configuration/).

## Contributing

If you want to **report a bug** that [does not exist yet](https://todo.sr.ht/~soywod/pimalaya), please send an email at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

If you want to **propose a feature** or **fix a bug**, please send a patch at [~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht) using [git send-email](https://git-scm.com/docs/git-send-email). Follow [this guide](https://git-send-email.io/) to configure git properly.

If you just want to **discuss** about the project, feel free to join the [Matrix](https://matrix.org/) workspace [#pimalaya.general](https://matrix.to/#/#pimalaya.general:matrix.org) or contact me directly [@soywod](https://matrix.to/#/@soywod:matrix.org). You can also use the mailing list [[send an email](mailto:~soywod/pimalaya@lists.sr.ht)|[subscribe](mailto:~soywod/pimalaya+subscribe@lists.sr.ht)|[unsubscribe](mailto:~soywod/pimalaya+unsubscribe@lists.sr.ht)].

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/project/Himalaya/index.html)

Special thanks to the [NLnet foundation](https://nlnet.nl/project/Himalaya/index.html) and the [European Commission](https://www.ngi.eu/) that helped the project to receive financial support from:

- [NGI Assure](https://nlnet.nl/assure/) in 2022
- [NGI Zero Untrust](https://nlnet.nl/entrust/) in 2023

If you appreciate the project, feel free to donate using one of the following providers:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
