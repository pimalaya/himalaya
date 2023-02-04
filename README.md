# 📫 Himalaya [![GitHub release](https://img.shields.io/github/v/release/soywod/himalaya?color=success&style=flat-square)](https://github.com/soywod/himalaya/releases/latest) [![Matrix](https://img.shields.io/matrix/himalaya.email.client:matrix.org?color=success&label=chat&style=flat-square)](https://matrix.to/#/#himalaya.email.client:matrix.org)

Command-line interface for email management based on the
[himalaya-lib](https://git.sr.ht/~soywod/himalaya-lib).

![image](https://user-images.githubusercontent.com/10437171/138774902-7b9de5a3-93eb-44b0-8cfb-6d2e11e3b1aa.png)

*Warning: the project is under active development, do not use in
production before the `v1.0.0`.*

## Features

- Folder listing
- Email listing and searching
- Email composition based on `$EDITOR`
- Email manipulation (copy/move/delete)
- Multi-accounting
- Account listing
- IMAP, Maildir and Notmuch support
- IMAP IDLE mode for real-time notifications
- PGP end-to-end encryption
- Completions for various shells
- JSON output
- …

*Note: see the [wiki](https://github.com/soywod/himalaya/wiki) for all
the features.*

## Installation

<table align="center">
<tr>
<td width="50%">
<a href="https://repology.org/project/himalaya/versions">
<img src="https://repology.org/badge/vertical-allrepos/himalaya.svg" alt="Packaging status" />
</a>
</td>
<td width="50%">

```shell
# Arch Linux (official)
$ pacman -S himalaya

# Arch Linux (from sources)
$ yay -S himalaya-git

# Homebrew
$ brew install himalaya

# Cargo
$ cargo install himalaya

# Nix
$ nix-env -i himalaya
```

*Note: see the
[wiki](https://github.com/soywod/himalaya/wiki/Installation) for other
installation methods.*

</td>
</tr>
</table>

## Configuration

```toml
# ~/.config/himalaya/config.toml

display-name = "Test"
downloads-dir = "~/downloads"
signature = "Regards,"

[gmail]
default = true
email = "test@gmail.com"

backend = "imap"
imap-host = "imap.gmail.com"
imap-port = 993
imap-login = "test@gmail.com"
imap-passwd-cmd = "security find-internet-password -gs gmail -w"

sender = "smtp"
smtp-host = "smtp.gmail.com"
smtp-port = 465
smtp-login = "test@gmail.com"
smtp-passwd-cmd = "security find-internet-password -gs gmail -w"

[gmail.folder-aliases]
inbox = "INBOX"
sent = "[Gmail]/Sent"
drafts = "[Gmail]/Drafts"

[local]
email = "test@localhost"
signature-delim = "~~\n"
signature = "Regards,"

backend = "maildir"
maildir-root-dir = "~/emails"

sender = "sendmail"
sendmail-cmd = "msmtp --read-envelope-from --read-recipients"
```

*Note: see the
[wiki](https://github.com/soywod/himalaya/wiki/Configuration) for all
the options.*

## Contributing

If you find a **bug**, please send an email at
[~soywod/himalaya@todo.sr.ht](mailto:~soywod/himalaya@todo.sr.ht).

If you have a **question**, please send an email at
[~soywod/himalaya@lists.sr.ht](mailto:~soywod/himalaya@lists.sr.ht).

If you want to **propose a feature** or **fix a bug**, please send a
patch at
[~soywod/himalaya@lists.sr.ht](mailto:~soywod/himalaya@lists.sr.ht)
using [git send-email](https://git-scm.com/docs/git-send-email) (see
[this guide](https://git-send-email.io/) on how to configure it).

If you want to **subscribe** to the mailing list, please send an email
at
[~soywod/himalaya+subscribe@lists.sr.ht](mailto:~soywod/himalaya+subscribe@lists.sr.ht).

If you want to **unsubscribe** to the mailing list, please send an
email at
[~soywod/himalaya+unsubscribe@lists.sr.ht](mailto:~soywod/himalaya+unsubscribe@lists.sr.ht).

If you want to **discuss** about the project, feel free to join the
[Matrix](https://matrix.org/) room
[#himalaya.email.client](https://matrix.to/#/#himalaya.email.client:matrix.org)
or contact me directly
[@soywod](https://matrix.to/#/@soywod:matrix.org).

## Credits

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/project/Himalaya/index.html)

Special thanks to the
[nlnet](https://nlnet.nl/project/Himalaya/index.html) foundation that
helped Himalaya to receive financial support from the [NGI
Assure](https://www.ngi.eu/ngi-projects/ngi-assure/) program of the
European Commission in September, 2022.

* [himalaya-lib](https://git.sr.ht/~soywod/himalaya-lib)
* [IMAP RFC3501](https://tools.ietf.org/html/rfc3501)
* [Iris](https://github.com/soywod/iris.vim), the himalaya predecessor
* [isync](https://isync.sourceforge.io/), an email synchronizer for
  offline usage
* [NeoMutt](https://neomutt.org/), an email terminal user interface
* [Alpine](http://alpine.x10host.com/alpine/alpine-info/), an other
  email terminal user interface
* [mutt-wizard](https://github.com/LukeSmithxyz/mutt-wizard), a tool
  over NeoMutt and isync
* [rust-imap](https://github.com/jonhoo/rust-imap), a Rust IMAP
  library
* [lettre](https://github.com/lettre/lettre), a Rust mailer library
* [mailparse](https://github.com/staktrace/mailparse), a Rust MIME
  email parser.

## Sponsoring

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors&style=flat-square)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff&style=flat-square)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff&style=flat-square)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000&style=flat-square)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222&style=flat-square)](https://liberapay.com/soywod)
