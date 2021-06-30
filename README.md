# 📫 Himalaya
[![gh-actions](https://github.com/soywod/himalaya/workflows/deployment/badge.svg)](https://github.com/soywod/himalaya/actions?query=workflow%3Adeployment)
[![gh-actions](https://github.com/soywod/himalaya/workflows/tests/badge.svg)](https://github.com/soywod/himalaya/actions?query=workflow%3Atests)
[![gh-actions](https://github.com/soywod/himalaya/workflows/nix-build/badge.svg)](https://github.com/soywod/himalaya/actions?query=workflow%3Anix-build)
[![Homebrew](https://img.shields.io/badge/dynamic/json.svg?url=https://formulae.brew.sh/api/formula/himalaya.json&query=$.versions.stable&label=homebrew)](https://formulae.brew.sh/formula/himalaya)

CLI email client written in Rust.

*The project is under active development. Do not use in production before the
`v1.0.0` (see the [roadmap](https://github.com/soywod/himalaya/milestone/5)).*

![image](https://user-images.githubusercontent.com/10437171/115144003-8a1b4880-a04a-11eb-80d2-245027e28591.png)

## Motivation

Bringing emails to the terminal is a *pain*. First, because they are sensitive
data. Secondly, the existing TUIs ([Mutt](http://www.mutt.org/),
[NeoMutt](https://neomutt.org/), [Alpine](https://alpine.x10host.com/),
[aerc](https://aerc-mail.org/)…) are really hard to configure. They require time
and patience.

The aim of Himalaya is to extract the email logic into a simple (yet solid) CLI
API that can be used directly from the terminal, from scripts, from UIs…
Possibilities are endless!

## Installation

```sh
# As root:
curl -sSL https://raw.githubusercontent.com/soywod/himalaya/master/install.sh | sudo sh

# As a regular user:
curl -sSL https://raw.githubusercontent.com/soywod/himalaya/master/install.sh | PREFIX=~/.local sh
```

*See the [wiki](https://github.com/soywod/himalaya/wiki) for other installation
methods.*

## Configuration

```toml
# ~/.config/himalaya/config.toml

name = "Your full name"
downloads-dir = "/abs/path/to/downloads"
signature = """
--
Regards,
"""

[gmail]
default = true
email = "your.email@gmail.com"

imap-host = "imap.gmail.com"
imap-port = 993
imap-login = "your.email@gmail.com"
imap-passwd-cmd = "pass show gmail"

smtp-host = "smtp.gmail.com"
smtp-port = 465
smtp-login = "your.email@gmail.com"
smtp-passwd-cmd = "security find-internet-password -gs gmail -w"
```

*See the
[wiki](https://github.com/soywod/himalaya/wiki/Configuration:config-file) for
all the options.*

## Features

- Mailbox listing
- Email listing and filtering
- Email composition based on `$EDITOR`
- Email manipulation (copy/move/delete)
- Multi-accounting
- IDLE mode for real-time notifications
- Vim plugin
- Completions for bash/zsh/fish
- JSON output
- …

*See the [wiki](https://github.com/soywod/himalaya/wiki) for all the features.*

## Sponsoring

[![github](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![paypal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
[![ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![buy-me-a-coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)

## Credits

- [IMAP RFC3501](https://tools.ietf.org/html/rfc3501)
- [Iris](https://github.com/soywod/iris.vim), the himalaya predecessor
- [isync](https://isync.sourceforge.io/), an email synchronizer for offline usage
- [NeoMutt](https://neomutt.org/), an email terminal user interface
- [Alpine](http://alpine.x10host.com/alpine/alpine-info/), an other email terminal user interface
- [mutt-wizard](https://github.com/LukeSmithxyz/mutt-wizard), a tool over NeoMutt and isync
- [rust-imap](https://github.com/jonhoo/rust-imap), a rust IMAP lib
