# 📫 Himalaya [![gh-actions](https://github.com/soywod/himalaya/workflows/deployment/badge.svg)](https://github.com/soywod/himalaya/actions?query=workflow%3Adeployment)

Minimalist CLI email client, written in Rust.

*The project is under active development. Do not use in production before the
`v1.0.0` (see the [roadmap](https://github.com/soywod/himalaya/projects/2)).*

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
  * [Copy a message](#copy-a-message)
  * [Move a message](#move-a-message)
  * [Delete a message](#delete-a-message)
  * [Listen to new messages](#listen-to-new-messages)
* [Completions](#completions)
* [Interfaces](#interfaces)
  * [GUI](#gui)
  * [TUI](#tui)
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

```sh
sh -c "$(curl -sSL https://github.com/soywod/himalaya/raw/master/install.sh)"
```

*See the [wiki section](https://github.com/soywod/himalaya/wiki/Installation)
for other installation methods.*

## Configuration

```toml
# ~/.config/himalaya/config.toml

name = "Your full name"
downloads-dir = "/abs/path/to/downloads"
signature = "Regards,"

[gmail]
default = true
email = "your.email@gmail.com"

imap-host = "imap.gmail.com"
imap-port = 993
imap-login = "your.email@gmail.com"
imap-passwd-cmd = "pass show gmail"

smtp-host = "smtp.gmail.com"
smtp-port = 487
smtp-login = "your.email@gmail.com"
smtp-passwd-cmd = "security find-internet-password -gs gmail -w"
```

*See the [wiki section](https://github.com/soywod/himalaya/wiki/Configuration)
for all the options.*

## Usage

```
himalaya 0.2.5
soywod <clement.douin@posteo.net>
📫 Minimalist CLI email client

USAGE:
    himalaya [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --account <STRING>     Selects a specific account
    -l, --log <LEVEL>          Defines the logs level [default: info]  [possible values: error, warn, info, debug,
                               trace]
    -m, --mailbox <MAILBOX>    Selects a specific mailbox [default: INBOX]
    -o, --output <FMT>         Defines the output format [default: plain]  [possible values: plain, json]

SUBCOMMANDS:
    attachments    Downloads all message attachments
    copy           Copy a message to the targetted mailbox
    delete         Delete a message
    flags          Handles flags
    forward        Forwards a message
    help           Prints this message or the help of the given subcommand(s)
    idle           Spawns a blocking idle daemon
    list           Lists all messages
    mailboxes      Lists all mailboxes
    move           Move a message to the targetted mailbox
    read           Reads text bodies of a message
    reply          Answers to a message
    save           Saves a raw message
    search         Lists messages matching the given IMAP query
    send           Sends a raw message
    template       Generates a message template
    write          Writes a new message
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

### Copy a message

```bash
himalaya copy 5123 Sent
```

Copies a message to the targetted mailbox.

### Move a message

```bash
himalaya move 5123 Drafts
```

Moves a message to the targetted mailbox.

### Delete a message

```bash
himalaya delete 5123
```

Moves a message.

### Listen to new messages

```bash
himalaya idle
```

Starts a session in idle mode (blocking). When a new message arrives, it runs
the command `notify-cmd` defined in the [config
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

## Completions

```sh
Generates the completion script for the given shell

USAGE:
    himalaya completion <shell>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <shell>     [possible values: bash, zsh, fish]
```

The command prints the generated script to the stdout. You will have
to manually save and source them. For example:

```sh
himalaya completion bash > himalaya-completions.bash
```

```sh
# ~/.bashrc

source himalaya-completions.bash
```

## Interfaces

### GUI

Not yet, but feel free to contribute ;)

### TUI

- [Vim plugin](https://github.com/soywod/himalaya/tree/master/vim)

## Credits

- [IMAP RFC3501](https://tools.ietf.org/html/rfc3501)
- [Iris](https://github.com/soywod/iris.vim), the himalaya predecessor
- [isync](https://isync.sourceforge.io/), an email synchronizer for offline usage
- [NeoMutt](https://neomutt.org/), an email terminal user interface
- [Alpine](http://alpine.x10host.com/alpine/alpine-info/), an other email terminal user interface
- [mutt-wizard](https://github.com/LukeSmithxyz/mutt-wizard), a tool over NeoMutt and isync
- [rust-imap](https://github.com/jonhoo/rust-imap), a rust IMAP lib
