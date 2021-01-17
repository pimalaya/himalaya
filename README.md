# 📫 Himalaya [![gh-actions](https://github.com/soywod/himalaya/workflows/deployment/badge.svg)](https://github.com/soywod/himalaya/actions?query=workflow%3Adeployment)

Minimalist CLI email client, written in Rust.

![image](https://user-images.githubusercontent.com/10437171/104848096-aee51000-58e3-11eb-8d99-bcfab5ca28ba.png)

## Table of contents

* [Motivation](#motivation)
* [Installation](#installation)
* [Usage](#usage)
  * [List mailboxes](#list-mailboxes)
  * [List emails](#list-emails)
  * [Search emails](#search-emails)
  * [Download email attachments](#download-email-attachments)
  * [Read email](#read-email)
  * [Reply email](#reply-email)
  * [Forward email](#forward-email)
* [License](https://github.com/soywod/himalaya/blob/master/LICENSE)
* [Changelog](https://github.com/soywod/himalaya/blob/master/CHANGELOG.md)
* [Credits](#credits)

## Motivation

Bringing emails to your terminal is a pain. The mainstream TUI, (neo)mutt,
takes time to configure. The default mapping is not intuitive when coming from
the Vim environment. It is even scary to use at the beginning, since you are
dealing with sensitive data!

The aim of Himalaya is to extract the email logic into a simple CLI API that
can be used either directly for the terminal or from various interfaces. It
gives users more flexibility.

## Installation

```bash
curl -sSL https://raw.githubusercontent.com/soywod/himalaya/master/install.sh | bash
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Installation) for
more information.*

## Configuration

```toml
# ~/.config/himalaya/config.toml

name = "Your full name"
downloads_dir = "/abs/path/to/downloads"

# Himalaya supports the multi-account
# Each account should be inside a TOML section
[gmail]
default = true
email = "my.email@gmail.com"

imap_host = "imap.gmail.com"
imap_port = 993
imap_login = "p.durant@gmail.test.com"
imap_passwd_cmd = "pass show gmail"

smtp_host = "smtp.gmail.com"
smtp_port = 487
smtp_login = "p.durant@gmail.test.com"
smtp_passwd_cmd = "pass show gmail"

[posteo]
name = "Your overriden full name"
downloads_dir = "/abs/path/to/overriden/downloads"
email = "my.email@posteo.net"

imap_host = "posteo.de"
imap_port = 993
imap_login = "my.email@posteo.net"
imap_passwd_cmd = "security find-internet-password -gs posteo -w"

smtp_host = "posteo.de"
smtp_port = 487
smtp_login = "my.email@posteo.net"
smtp_passwd_cmd = "security find-internet-password -gs posteo -w"

# [other account]
# ...
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Configuration) for
more information.*

## Usage

```
Himalaya 0.1.0
soywod <clement.douin@posteo.net>
📫 Minimalist CLI email client

USAGE:
    himalaya [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --account <STRING>    Name of the config file to use

SUBCOMMANDS:
    attachments    Downloads all attachments from an email
    forward        Forwards an email
    help           Prints this message or the help of the given subcommand(s)
    list           Lists emails sorted by arrival date
    mailboxes      Lists all available mailboxes
    read           Reads text bodies of an email
    reply          Answers to an email
    search         Lists emails matching the given IMAP query
    write          Writes a new email
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Usage) for more
information.*

### List mailboxes

![image](https://user-images.githubusercontent.com/10437171/104848169-0e432000-58e4-11eb-8410-05f0404c0d99.png)

```
himalaya-mailboxes 
Lists all available mailboxes

USAGE:
    himalaya mailboxes

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Usage:mailboxes)
for more information.*

### List emails

![image](https://user-images.githubusercontent.com/10437171/104848096-aee51000-58e3-11eb-8d99-bcfab5ca28ba.png)

```
himalaya-list 
Lists emails sorted by arrival date

USAGE:
    himalaya list [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -m, --mailbox <STRING>    Name of the mailbox [default: INBOX]
    -p, --page <INT>          Page number [default: 0]
    -s, --size <INT>          Page size [default: 10]
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Usage:list) for
more information.*

### Search emails

![image](https://user-images.githubusercontent.com/10437171/104848096-aee51000-58e3-11eb-8d99-bcfab5ca28ba.png)

```
himalaya-search 
Lists emails matching the given IMAP query

USAGE:
    himalaya search [OPTIONS] <QUERY>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -m, --mailbox <STRING>    Name of the mailbox [default: INBOX]
    -p, --page <INT>          Page number [default: 0]
    -s, --size <INT>          Page size [default: 10]

ARGS:
    <QUERY>...    IMAP query (see https://tools.ietf.org/html/rfc3501#section-6.4.4)
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Usage:search) for
more information.*

### Download email attachments

![image](https://user-images.githubusercontent.com/10437171/104848278-890c3b00-58e4-11eb-9b5c-48807c04f762.png)

```
himalaya-attachments 
Downloads all attachments from an email

USAGE:
    himalaya attachments [OPTIONS] <UID>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -m, --mailbox <STRING>    Name of the mailbox [default: INBOX]

ARGS:
    <UID>    UID of the email
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Usage:attachments)
for more information.*

### Read email

```
himalaya-read 
Reads text bodies of an email

USAGE:
    himalaya read [OPTIONS] <UID>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -m, --mailbox <STRING>      Name of the mailbox [default: INBOX]
    -t, --mime-type <STRING>    MIME type to use [default: plain]  [possible values: plain, html]

ARGS:
    <UID>    UID of the email
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Usage:read) for
more information.*

### Write email

```
himalaya-write 
Writes a new email

USAGE:
    himalaya write

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Usage:write) for
more information.*

### Reply email

```
himalaya-reply 
Answers to an email

USAGE:
    himalaya reply [FLAGS] [OPTIONS] <UID>

FLAGS:
    -h, --help       Prints help information
    -a, --all        Includs all recipients
    -V, --version    Prints version information

OPTIONS:
    -m, --mailbox <STRING>    Name of the mailbox [default: INBOX]

ARGS:
    <UID>    UID of the email
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Usage:reply) for
more information.*

### Forward email

```
himalaya-forward 
Forwards an email

USAGE:
    himalaya forward [OPTIONS] <UID>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -m, --mailbox <STRING>    Name of the mailbox [default: INBOX]

ARGS:
    <UID>    UID of the email
```

*See [wiki section](https://github.com/soywod/himalaya/wiki/Usage:forward) for
more information.*

## Credits

- [IMAP RFC3501](https://tools.ietf.org/html/rfc3501)
- [Iris](https://github.com/soywod/iris.vim), the himalaya predecessor
- [Neomutt](https://neomutt.org/)
- [Alpine](http://alpine.x10host.com/alpine/alpine-info/)
- [rust-imap](https://github.com/jonhoo/rust-imap)
