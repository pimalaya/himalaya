<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI to manage emails,<br>based on <a href="https://crates.io/crates/email-lib"><code>email-lib</code></a>.</p>
  <p>
    <a href="https://github.com/soywod/neverest/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/soywod/neverest?color=success"/></a>
	<a href="https://repology.org/project/himalaya/versions"><img alt="Repology" src="https://img.shields.io/repology/repositories/himalaya"></a>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/matrix/pimalaya:matrix.org?color=success&label=chat"/></a>
  </p>
  <!-- <p><em>🚧 <strong>Work In Progress</strong>, stay tuned! 🚧</em></p> -->
</div>

```
$ himalaya envelope list --account posteo --folder Archives.FOSS --page 2
```

![screenshot](./screenshot)

## Features

- [Folder (aka mailbox) management](https://pimalaya.org/himalaya/cli/latest/usage/advanced/folder/)
- Envelope [listing](https://pimalaya.org/himalaya/cli/latest/usage/basic/envelope/list.html), [filtering and sorting](https://pimalaya.org/himalaya/cli/latest/usage/advanced/envelope/list.html)
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
