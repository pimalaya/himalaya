<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI to manage emails</p>
  <p>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white"/></a>
    <a href="https://fosstodon.org/@pimalaya"><img alt="Mastodon" src="https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white"/></a>
    <a href="https://pimalaya.org/sponsor/"><img alt="Sponsor" src="https://img.shields.io/badge/sponsor-pink?style=flat&logo=github-sponsors&logoColor=white"/></a>
  </p>
</div>

```sh
$ himalaya envelope search --json --sort newest-first from:alice date:2026-11-02..
```

```json
{
  "envelopes": [
    {
      "id": "2",
      "messageId": "slides@example.org",
      "inReplyTo": [],
      "flags": [],
      "subject": "Slides for the review",
      "from": [{ "name": "Alice", "email": "alice@example.org" }],
      "to": [{ "name": null, "email": "me@example.org" }],
      "date": "2026-11-19T09:15:00+01:00",
      "size": 355,
      "hasAttachment": true
    },
    {
      "id": "1",
      "messageId": "lunch@example.org",
      "inReplyTo": [],
      "flags": [{ "raw": "\\Seen", "iana": "seen" }],
      "subject": "Lunch on Friday?",
      "from": [{ "name": "Alice", "email": "alice@example.org" }],
      "to": [{ "name": null, "email": "me@example.org" }],
      "date": "2026-11-18T12:30:00+01:00",
      "size": 162,
      "hasAttachment": false
    }
  ],
  "nextPage": null,
  "total": null,
  "errors": []
}
```

## Table of contents

- [Features](#features)
- [Installation](#installation)
- [Configuration](#configuration)
  - [Proton Mail](#proton-mail)
  - [Fastmail](#fastmail)
  - [Gmail](#gmail)
  - [Outlook](#outlook)
  - [Posteo](#posteo)
  - [iCloud Mail](#icloud-mail)
- [Usage](#usage)
- [Alternatives](#alternatives)
- [AI policy](https://github.com/pimalaya/.github/blob/master/AI_POLICY.md)
- [License](#license)
- [Social](#social)
- [Contributing](./CONTRIBUTING.md)
- [Sponsoring](#sponsoring)

## Features

- **JSON output** of every command, described by `himalaya json-schema`, for scripts, frontends and agents
- **Unified API** for mailboxes, envelopes, flags, messages and attachments, the same on every backend
- **Protocol-specific APIs** exposing the full surface of IMAP, SMTP, ManageSieve, JMAP, Gmail and Microsoft Graph
- **Local stores**: Maildir, mbox and pimdir, the offline store [Neverest](https://github.com/pimalaya/neverest) syncs
- **One query language** to search envelopes, whatever the backend
- **REPL** and **MCP server** running commands over connections kept for the session
- **TOML configuration** with several accounts, secrets read from any password manager
- **Plain text** output for a person: one item per line, no border, no colour

## Installation

<details>
  <summary>Debian, Ubuntu</summary>

  Himalaya is not in the official repositories yet, but the signed APT repository of [pkg.pimalaya.org](https://pkg.pimalaya.org) ships the latest release with its man pages and shell completions. Add the repository once:

  ```sh
  sudo install -d -m 0755 /etc/apt/keyrings
  curl -fsSL https://pkg.pimalaya.org/pimalaya.asc | sudo tee /etc/apt/keyrings/pimalaya.asc >/dev/null
  sudo tee /etc/apt/sources.list.d/pimalaya.sources >/dev/null <<EOF
  Types: deb
  URIs: https://pkg.pimalaya.org/apt
  Suites: stable
  Components: main
  Signed-By: /etc/apt/keyrings/pimalaya.asc
  EOF
  sudo apt update
  ```

  Then install Himalaya:

  ```sh
  sudo apt install himalaya
  ```
</details>

<details>
  <summary>Fedora, RHEL, Rocky Linux, AlmaLinux</summary>

  Himalaya is not in the official repositories yet, but the signed DNF repository of [pkg.pimalaya.org](https://pkg.pimalaya.org) ships the latest release with its man pages and shell completions:

  ```sh
  sudo curl -fsSL -o /etc/yum.repos.d/pimalaya.repo https://pkg.pimalaya.org/pimalaya.repo
  sudo dnf install himalaya
  ```
</details>

<details>
  <summary>Homebrew (macOS, Linux)</summary>

  ```sh
  brew install himalaya
  ```
</details>

<details>
  <summary>Pre-built binary</summary>

  For other systems, Himalaya can be installed with the installer:

  *As root:*

  ```sh
  curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh
  ```

  *As a regular user:*

  ```sh
  curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
  ```

  These commands install the latest [release](https://github.com/pimalaya/himalaya/releases), built with the default cargo features. For a build of `master`, check out the [releases](https://github.com/pimalaya/himalaya/actions/workflows/releases.yml) GitHub workflow and look for the *Artifacts* section.
</details>

<details>
  <summary>Cargo</summary>

  ```sh
  cargo install --locked himalaya
  ```

  For a build of `master`:

  ```sh
  cargo install --locked --git https://github.com/pimalaya/himalaya.git
  ```
</details>

<details>
  <summary>Nix</summary>

  ```sh
  nix profile install github:pimalaya/himalaya
  ```
</details>

<details>
  <summary>Sources</summary>

  Requires [Rust](https://www.rust-lang.org/tools/install) 1.89 or later:

  ```sh
  git clone https://github.com/pimalaya/himalaya
  cd himalaya
  cargo build --release --locked
  ```

  The binary lands at target/release/himalaya.
</details>

## Configuration

A TOML configuration is loaded from every existing path among the following, each one deep-merged on top of the ones before:

- `$XDG_CONFIG_HOME/himalaya/config.toml`
- `$HOME/.config/himalaya/config.toml`
- `$HOME/.himalayarc`

Override the path with `-c <PATH>`, repeated to merge several files. The same file backs [himalaya-tui](https://github.com/pimalaya/himalaya-tui), its own keys under `[tui]`.

> [!TIP]
> See [config.sample.toml](./config.sample.toml) for a complete configuration of one example account with commented fields; rename the account and uncomment what you need, or start from a provider below.

### Proton Mail

Run [Proton Bridge](https://proton.me/mail/bridge), which serves Proton on a local IMAP and SMTP endpoint; the password is the one the Bridge generates, not your account's.

```toml
[accounts.proton]

imap.server = "imap://127.0.0.1:1143"
imap.sasl.plain.username = "example@proton.me"
imap.sasl.plain.password.command = "pass show proton-bridge"

smtp.server = "smtp://127.0.0.1:1025"
smtp.sasl.plain.username = "example@proton.me"
smtp.sasl.plain.password.command = "pass show proton-bridge"
```

### Fastmail

Use an [app password](https://www.fastmail.help/hc/en-us/articles/360058752854) for IMAP and SMTP, or an [API token](https://www.fastmail.help/hc/en-us/articles/5254602856719) for JMAP, replacing both blocks.

```toml
[accounts.fastmail]

imap.server = "imaps://imap.fastmail.com"
imap.sasl.plain.username = "example@fastmail.com"
imap.sasl.plain.password.command = "pass show fastmail"

smtp.server = "smtps://smtp.fastmail.com"
smtp.sasl.plain.username = "example@fastmail.com"
smtp.sasl.plain.password.command = "pass show fastmail"

# jmap.server = "https://api.fastmail.com/jmap/session"
# jmap.auth.bearer.token.command = "pass show fastmail"
```

### Gmail

Use an [app password](https://myaccount.google.com/apppasswords) for IMAP and SMTP, or an OAuth 2.0 token from [Ortie](https://github.com/pimalaya/ortie) for the Gmail API, replacing both blocks. Special mailboxes live under `[Gmail]/`, so alias them.

```toml
[accounts.gmail]

imap.server = "imaps://imap.gmail.com:993"
imap.sasl.plain.username = "example@gmail.com"
imap.sasl.plain.password.command = "pass show gmail"

smtp.server = "smtps://smtp.gmail.com:465"
smtp.sasl.plain.username = "example@gmail.com"
smtp.sasl.plain.password.command = "pass show gmail"

mailbox.alias.sent = "[Gmail]/Sent Mail"
mailbox.alias.drafts = "[Gmail]/Drafts"
mailbox.alias.trash = "[Gmail]/Trash"
mailbox.alias.archive = "[Gmail]/All Mail"

# gmail.auth.token.command = ["ortie", "token", "show", "-a", "gmail"]
```

### Outlook

Microsoft refuses passwords: use `xoauth2` with a token from [Ortie](https://github.com/pimalaya/ortie), or the Microsoft Graph API, replacing both blocks.

```toml
[accounts.outlook]

imap.server = "imaps://outlook.office365.com:993"
imap.sasl.xoauth2.username = "example@outlook.com"
imap.sasl.xoauth2.token.command = ["ortie", "token", "show", "-a", "outlook"]

smtp.server = "smtp://smtp-mail.outlook.com:587"
smtp.starttls = true
smtp.sasl.xoauth2.username = "example@outlook.com"
smtp.sasl.xoauth2.token.command = ["ortie", "token", "show", "-a", "outlook"]

# msgraph.auth.token.command = ["ortie", "token", "show", "-a", "msgraph"]
```

### Posteo

Standard IMAP and SMTP with the account password.

```toml
[accounts.posteo]

imap.server = "imaps://posteo.de"
imap.sasl.plain.username = "example@posteo.net"
imap.sasl.plain.password.command = "pass show posteo"

smtp.server = "smtps://posteo.de"
smtp.sasl.plain.username = "example@posteo.net"
smtp.sasl.plain.password.command = "pass show posteo"
```

### iCloud Mail

Use an [app-specific password](https://support.apple.com/en-us/HT204397); the IMAP login is the address's name, the SMTP login the full address.

```toml
[accounts.icloud]

imap.server = "imaps://imap.mail.me.com:993"
imap.sasl.plain.username = "johnappleseed"
imap.sasl.plain.password.command = "pass show icloud"

smtp.server = "smtp://smtp.mail.me.com:587"
smtp.starttls = true
smtp.sasl.plain.username = "johnappleseed@icloud.com"
smtp.sasl.plain.password.command = "pass show icloud"

mailbox.alias.sent = "Sent Messages"
```

## Usage

**Step 1**: once your first account is configured, check that every backend answers:

```sh
himalaya account check
```

**Step 2**: list the envelopes of the inbox:

```sh
himalaya envelope list
```

**Step 3**: read a message by its id:

```sh
himalaya message read 42
```

Every command runs on the default account unless `-a` names another, and its `--help` details its flags, exit codes and JSON output. More use cases:

```sh
# Search the newest messages from Alice of the last two days
himalaya envelope search --sort newest-first from:alice date:2d

# Reply and send at once
himalaya message reply 42 --body "Thanks!" --send

# Mark messages as seen
himalaya flag add --flag seen 1 2 3

# Send a raw IMAP command
himalaya imap raw 'a1 NAMESPACE'

# Serve the account to an AI agent, writes allowed
himalaya mcp --allow-write
```

## Alternatives

- [NeoMutt](https://neomutt.org/): a terminal mail client, driven from its own interface rather than scripted
- [aerc](https://aerc-mail.org/): a terminal mail client for IMAP, JMAP, Maildir and notmuch
- [mblaze](https://github.com/leahneukirchen/mblaze): Unix utilities over a Maildir, local mail only

## License

This project is licensed under either of:

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

## Social

- Chat on [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- News on [Mastodon](https://fosstodon.org/@pimalaya) or [RSS](https://fosstodon.org/@pimalaya.rss)
- Mail at [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Special thanks to the [NLnet foundation](https://nlnet.nl/) and the [European Commission](https://www.ngi.eu/) that have been financially supporting the project for years:

- 2022 → 2023: [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024: [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026: [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- 2026 → 2027: [NGI Zero Commons Fund](https://nlnet.nl/project/Pimalaya-pimdir/)

This program is part of Pimalaya, free software funded entirely by grants and donations. If you find it useful, consider [sponsoring](https://pimalaya.org/sponsor/) its development:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/pimalaya)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/pimalaya)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/pimalaya)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/u/gh/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
