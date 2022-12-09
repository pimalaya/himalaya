# 📫 Himalaya

Command-line interface for email management based on the
[himalaya-lib](https://git.sr.ht/~soywod/himalaya-lib).

![image](https://user-images.githubusercontent.com/10437171/138774902-7b9de5a3-93eb-44b0-8cfb-6d2e11e3b1aa.png)

*The project is under active development. Do not use in production
before the `v1.0.0`.*

## Installation

[![Packaging
status](https://repology.org/badge/vertical-allrepos/himalaya.svg)](https://repology.org/project/himalaya/versions)

```sh
curl -sSL https://raw.githubusercontent.com/soywod/himalaya/master/install.sh | PREFIX=~/.local sh
```

*See the
[wiki](https://github.com/soywod/himalaya/wiki/Installation:binary)
for other installation methods.*

## Configuration

```toml
# ~/.config/himalaya/config.toml

display-name = "Test"
downloads-dir = "~/downloads"
signature = "Regards,"

[gmail]
default = true
email = "test@gmail.com"

backend = "imap" # imap, maildir or notmuch
imap-host = "imap.gmail.com"
imap-port = 993
imap-login = "test@gmail.com"
imap-passwd-cmd = "pass show gmail"

sender = "smtp" # smtp or sendmail
smtp-host = "smtp.gmail.com"
smtp-port = 465
smtp-login = "test@gmail.com"
smtp-passwd-cmd = "security find-internet-password -gs gmail -w"
```

*See the
[wiki](https://github.com/soywod/himalaya/wiki/Configuration:config-file)
for all the options.*

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
- Vim and Emacs plugins
- Completions for various shells
- JSON output
- …

*See the
[wiki](https://github.com/soywod/himalaya/wiki/Usage:email:list) for
all the features.*

## Contributing

If you find a bug, feel free to open an issue at
[soywod/himalaya](https://github.com/soywod/himalaya/issues/new).

If you want to discuss about the project, feel free to open a [new
discussion](https://github.com/soywod/himalaya/discussions/new). You
can also join the [Matrix](https://matrix.org/) room
[#himalaya.email.client](https://matrix.to/#/#himalaya.email.client:matrix.org)
or contact me directly
[@soywod](https://matrix.to/#/@soywod:matrix.org).

## Sponsoring

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors&style=flat-square)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff&style=flat-square)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff&style=flat-square)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000&style=flat-square)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222&style=flat-square)](https://liberapay.com/soywod)

## Credits

- [himalaya-lib](https://git.sr.ht/~soywod/himalaya-lib)
- [IMAP RFC3501](https://tools.ietf.org/html/rfc3501)
- [Iris](https://github.com/soywod/iris.vim), the himalaya predecessor
- [isync](https://isync.sourceforge.io/), an email synchronizer for
  offline usage
- [NeoMutt](https://neomutt.org/), an email terminal user interface
- [Alpine](http://alpine.x10host.com/alpine/alpine-info/), an other
  email terminal user interface
- [mutt-wizard](https://github.com/LukeSmithxyz/mutt-wizard), a tool
  over NeoMutt and isync
- [rust-imap](https://github.com/jonhoo/rust-imap), a rust IMAP lib
