# 📫 Himalaya [![gh-actions](https://github.com/soywod/himalaya/workflows/deployment/badge.svg)](https://github.com/soywod/himalaya/actions?query=workflow%3Adeployment)

Minimalist CLI email client, written in Rust.

![image](https://user-images.githubusercontent.com/10437171/104848096-aee51000-58e3-11eb-8d99-bcfab5ca28ba.png)

## Table of contents

* [Motivation](#motivation)
* [Installation](#installation)
* [Configuration](#configuration)
* [Usage](#usage)
  * [List mailboxes](#list-mailboxes)
  * [List messages](#list-messages)
  * [Search messages](#search-messages)
  * [Download attachments](#download-attachments)
  * [Read a message](#read-a-message)
  * [Write a new message](#write-a-new-message)
  * [Reply to a message](#reply-to-a-message)
  * [Forward a message](#forward-a-message)
  * [Listen to new messages](#listen-to-new-messages)
* [License](https://github.com/soywod/himalaya/blob/master/LICENSE)
* [Changelog](https://github.com/soywod/himalaya/blob/master/CHANGELOG.md)
* [Credits](#credits)

## Motivation

Bringing emails to the terminal is a pain. The mainstream TUI, (neo)mutt, takes
time to configure. The default mapping is not intuitive when coming from the
Vim environment. It is even scary to use at the beginning, since you are
dealing with sensitive data!

The aim of Himalaya is to extract the email logic into a simple (yet solid) CLI
API that can be used either directly from the terminal or UIs. It gives users
more flexibility.

## Installation

```bash
curl -sSL https://raw.githubusercontent.com/soywod/himalaya/master/install.sh | bash
```

*See the [wiki section](https://github.com/soywod/himalaya/wiki/Installation)
for other installation methods.*

## Configuration

```toml
# ~/.config/himalaya/config.toml

name = "Your full name"
downloads-dir = "/abs/path/to/downloads"

[gmail]
default = true
email = "my.email@gmail.com"

imap-host = "imap.gmail.com"
imap-port = 993
imap-login = "test@gmail.com"
imap-passwd-cmd = "pass show gmail"

smtp-host = "smtp.gmail.com"
smtp-port = 487
smtp-login = "test@gmail.com"
smtp-passwd-cmd = "security find-internet-password -gs posteo -w"
```

*See the [wiki section](https://github.com/soywod/himalaya/wiki/Configuration)
for all the options.*

## Usage

```
Himalaya 0.2.0
soywod <clement.douin@posteo.net>
📫 Minimalist CLI email client

USAGE:
    himalaya [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --account <STRING>    Name of the account to use
    -o, --output <STRING>     Format of the output to print [possible values: text, json]

SUBCOMMANDS:
    attachments    Downloads all attachments from an email
    forward        Forwards an email
    help           Prints this message or the help of the given subcommand(s)
    idle           Starts the idle mode
    list           Lists emails sorted by arrival date
    mailboxes      Lists all available mailboxes
    read           Reads text bodies of an email
    reply          Answers to an email
    save           Saves a raw message in the given mailbox
    search         Lists emails matching the given IMAP query
    send           Sends a raw message
    template       Generates a message template
    write          Writes a new email
```

*See the [wiki section](https://github.com/soywod/himalaya/wiki/Usage) for more
information about commands.*

### List mailboxes

![image](https://user-images.githubusercontent.com/10437171/104848169-0e432000-58e4-11eb-8410-05f0404c0d99.png)

Shows mailboxes in a basic table.

### List messages

![image](https://user-images.githubusercontent.com/10437171/104848096-aee51000-58e3-11eb-8d99-bcfab5ca28ba.png)

Shows messages in a basic table.

### Search messages

![image](https://user-images.githubusercontent.com/10437171/110698977-9d86f880-81ee-11eb-8990-0ca89c7d4640.png)

Shows filtered messages in a basic table. The query should follow the
[RFC-3501](https://tools.ietf.org/html/rfc3501#section-6.4.4).

### Download attachments

![image](https://user-images.githubusercontent.com/10437171/104848278-890c3b00-58e4-11eb-9b5c-48807c04f762.png)

Downloads all attachments from a message directly to the
[`downloads-dir`](https://github.com/soywod/himalaya/wiki/Configuration).

### Read a message

![image](https://user-images.githubusercontent.com/10437171/110701369-5d754500-81f1-11eb-932f-94c2ca8db068.png)

Shows the text content of a message (`text/plain` if exists, otherwise
`text/html`).

### Write a new message

```bash
himalaya write
```

Opens your default editor (from the `$EDITOR` environment variable) to compose
a new message.

### Reply to a message

```bash
himalaya reply --all 5123
```

Opens your default editor to reply to a message.

### Forward a message

```bash
himalaya forward 5123
```

Opens your default editor to forward a message. 

### Listen to new messages

```bash
himalaya idle
```

Starts a session in idle mode (blocking). When a new message arrives, it runs
the command `notification-cmd` defined in the [config
file](https://github.com/soywod/himalaya/wiki/Configuration).

Here a use case with [`systemd`](https://en.wikipedia.org/wiki/Systemd):

```ini
# ~/.config/systemd/user/himalaya.service

[Unit]
Description=Himalaya new messages notifier

[Service]
ExecStart=/usr/local/bin/himalaya idle
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
systemctl --user enable himalaya.service
systemctl --user start  himalaya.service
```

## Credits

- [IMAP RFC3501](https://tools.ietf.org/html/rfc3501)
- [Iris](https://github.com/soywod/iris.vim), the himalaya predecessor
- [isync](https://isync.sourceforge.io/), an email synchronizer for offline usage
- [NeoMutt](https://neomutt.org/), an email terminal user interface
- [Alpine](http://alpine.x10host.com/alpine/alpine-info/), an other email terminal user interface
- [mutt-wizard](https://github.com/LukeSmithxyz/mutt-wizard), a tool over NeoMutt and isync
- [rust-imap](https://github.com/jonhoo/rust-imap), a rust IMAP lib
