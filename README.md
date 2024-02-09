# 📫 Himalaya [![GitHub release](https://img.shields.io/github/v/release/soywod/himalaya?color=success)](https://github.com/soywod/himalaya/releases/latest) [![Matrix](https://img.shields.io/matrix/pimalaya.himalaya:matrix.org?color=success&label=chat)](https://matrix.to/#/#pimalaya.himalaya:matrix.org)

Welcome to [**Himalaya CLI**](https://pimalaya.org/himalaya/cli/latest/), the Command-Line Interface to manage emails based on [email-lib](https://crates.io/crates/email-lib).

![screenshot](https://github.com/soywod/himalaya/assets/10437171/8a62cf1d-920e-4110-9849-170db6dc51ce)

*Disclaimer: the project is under active development, do not use in production before the final `v1.0.0`.*

## Features

- [Folder (aka mailbox) management](https://pimalaya.org/himalaya/cli/latest/usage/advanced/folder/)
- [Envelopes listing](https://pimalaya.org/himalaya/cli/latest/usage/basic/envelope/list.html)
- [Message composition](https://pimalaya.org/himalaya/cli/latest/usage/basic/message/send.html) based on `$EDITOR`
- Message manipulation ([copy](https://pimalaya.org/himalaya/cli/latest/usage/advanced/message/copy.html)/[move](https://pimalaya.org/himalaya/cli/latest/usage/advanced/message/move.html)/[delete](https://pimalaya.org/himalaya/cli/latest/usage/advanced/message/delete.html))
- [Multi-accounting](https://pimalaya.org/himalaya/cli/latest/configuration/)
- [Account synchronization](https://pimalaya.org/himalaya/cli/latest/usage/basic/account/sync.html) for offline usage
- Support multiple backends: [IMAP](https://pimalaya.org/himalaya/cli/latest/usage/advanced/imap.html), [Maildir](https://pimalaya.org/himalaya/cli/latest/usage/advanced/maildir.html), [Notmuch](https://pimalaya.org/himalaya/cli/latest/usage/advanced/notmuch.html), [SMTP](https://pimalaya.org/himalaya/cli/latest/usage/advanced/smtp.html), [Sendmail](https://pimalaya.org/himalaya/cli/latest/usage/advanced/sendmail.html).
- [PGP](https://pimalaya.org/himalaya/cli/latest/usage/advanced/pgp/) end-to-end encryption
- Generate [man pages](https://pimalaya.org/himalaya/cli/latest/usage/advanced/man.html)
- Generate [completion scripts](https://pimalaya.org/himalaya/cli/latest/usage/advanced/completion.html) for various shells
- [JSON output](https://pimalaya.org/himalaya/cli/latest/usage/advanced/#-o--output)
- …and more! [Get started now](https://pimalaya.org/himalaya/cli/latest/quickstart.html)

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

*See the [documentation](https://pimalaya.org/himalaya/cli/latest/installation.html) for other installation methods.*

</td>
</tr>
</table>

## Configuration

*Please read the [documentation](https://pimalaya.org/himalaya/cli/latest/configuration/).*

## Contributing

*Please read the [contributing guide](https://github.com/soywod/himalaya/blob/master/CONTRIBUTING.md) for more detailed information.*

A **bug tracker** is available on [SourceHut](https://todo.sr.ht/~soywod/pimalaya). <sup>[[send an email](mailto:~soywod/pimalaya@todo.sr.ht)]</sup>

A **mailing list** is available on [SourceHut](https://lists.sr.ht/~soywod/pimalaya). <sup>[[send an email](mailto:~soywod/pimalaya@lists.sr.ht)] [[subscribe](mailto:~soywod/pimalaya+subscribe@lists.sr.ht)] [[unsubscribe](mailto:~soywod/pimalaya+unsubscribe@lists.sr.ht)]</sup>

If you want to **report a bug**, please send an email at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

If you want to **propose a feature** or **fix a bug**, please send a patch at [~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht). The simplest way to send a patch is to use [git send-email](https://git-scm.com/docs/git-send-email), follow [this guide](https://git-send-email.io/) to configure git properly.

If you just want to **discuss** about the project, feel free to join the [Matrix](https://matrix.org/) workspace [#pimalaya.himalaya](https://matrix.to/#/#pimalaya.himalaya:matrix.org) or contact me directly [@soywod](https://matrix.to/#/@soywod:matrix.org). You can also use the mailing list.

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
