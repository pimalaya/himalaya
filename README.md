<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI to manage emails</p>
  <p>
    <a href="https://github.com/pimalaya/himalaya/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/pimalaya/himalaya?color=success"/></a>
    <a href="https://repology.org/project/himalaya/versions"><img alt="Repology" src="https://img.shields.io/repology/repositories/himalaya?color=success"></a>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white"/></a>
    <a href="https://fosstodon.org/@pimalaya"><img alt="Mastodon" src="https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white"/></a>
  </p>
</div>

```
himalaya envelopes list --account posteo -m Archives.FOSS --page 2
```

![screenshot](./screenshot.jpeg)

> [!IMPORTANT]
> This README documents Himalaya v2, which is **not yet released**. If you are running v1 (`himalaya v1.2.0` or earlier), refer to the [v1.2.0 README](https://github.com/pimalaya/himalaya/blob/v1.2.0/README.md) instead. The [MIGRATION.md](./MIGRATION.md) guide walks v1 users through the breaking changes.

## Table of contents

- [Features](#features)
- [Installation](#installation)
  - [Pre-built binary](#pre-built-binary)
  - [Cargo](#cargo)
  - [Arch Linux](#arch-linux)
  - [Homebrew](#homebrew)
  - [Scoop](#scoop)
  - [Fedora Linux/CentOS/RHEL](#fedora-linuxcentosrhel)
  - [Nix](#nix)
  - [Sources](#sources)
- [Configuration](#configuration)
- [Usage](#usage)
  - [Shared API](#shared-api)
  - [Protocol-specific APIs](#protocol-specific-apis)
  - [Composing messages](#composing-messages)
  - [Reading messages](#reading-messages)
  - [Re-using sessions](#re-using-sessions)
- [Interfaces](#interfaces)
- [FAQ](#faq)
- [Social](#social)
- [Sponsoring](#sponsoring)

## Features

- **Shared API** that maps `mailboxes`, `envelopes`, `flags`, `messages` and `attachments` to the active backend (IMAP, JMAP or Maildir)
- **Protocol-specific APIs** exposing each backend's full surface (`himalaya imap …`, `himalaya jmap …`, `himalaya maildir …`, `himalaya smtp …`)
- **IMAP** backend <sup>[rfc9051](https://www.iana.org/go/rfc9051)</sup> (requires `imap` cargo feature)
- **JMAP** backend <sup>[rfc8620](https://www.iana.org/go/rfc8620), [rfc8621](https://www.iana.org/go/rfc8621)</sup> (requires `jmap` cargo feature)
- **Maildir** backend (requires `maildir` cargo feature)
- **SMTP** backend <sup>[rfc5321](https://www.iana.org/go/rfc5321)</sup> (requires `smtp` cargo feature)
- **TLS** support:
  - [native-tls](https://crates.io/crates/native-tls) (requires `native-tls` feature)
  - [rustls](https://crates.io/crates/rustls):
    - AWS-LC crypto provider (requires `rustls-aws` feature)
    - Ring crypto provider (requires `rustls-ring` feature)
- **SASL** support: ANONYMOUS, LOGIN, PLAIN (IMAP/SMTP)
- **Provider discovery** wizard powered by [io-discovery](https://github.com/pimalaya/io-discovery): Thunderbird Autoconfiguration, PACC and RFC 6186 SRV lookups
- **TOML** configuration with multi-account support
- **JSON** output via `--json`

*Himalaya CLI is written in [Rust](https://www.rust-lang.org/), and relies on [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) to enable or disable functionalities. Default features can be found in the `features` section of the [`Cargo.toml`](./Cargo.toml#L18), or on [docs.rs](https://docs.rs/crate/himalaya/latest/features).*

## Installation

### Pre-built binary

Himalaya CLI can be installed with the `install.sh` installer:

*As root:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh
```

*As a regular user:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
```

These commands install the latest binary from the GitHub [releases](https://github.com/pimalaya/himalaya/releases) section.

If you want a more up-to-date version than the latest release, check out the [releases](https://github.com/pimalaya/himalaya/actions/workflows/releases.yml) GitHub workflow and look for the *Artifacts* section. You will find a pre-built binary matching your OS. These pre-built binaries are built from the `master` branch.

*Such binaries are built with the default cargo features. If you need more features, please use another installation method.*

### Cargo

Himalaya CLI can be installed with [cargo](https://doc.rust-lang.org/cargo/):

```
cargo install himalaya --locked
```

With only IMAP support:

```
cargo install himalaya --locked --no-default-features --features imap
```

You can also use the git repository for a more up-to-date (but less stable) version:

```
cargo install --locked --git https://github.com/pimalaya/himalaya.git
```

### Arch Linux

Himalaya CLI can be installed on [Arch Linux](https://archlinux.org/) with either the community repository:

```
pacman -S himalaya
```

or the [user repository](https://aur.archlinux.org/):

```
git clone https://aur.archlinux.org/himalaya-git.git
cd himalaya-git
makepkg -isc
```

If you use [yay](https://github.com/Jguer/yay), it is even simplier:

```
yay -S himalaya-git
```

### Homebrew

Himalaya CLI can be installed with [Homebrew](https://brew.sh/):

```
brew install himalaya
```

Note: cargo features are not compatible with brew. If you need a different feature set, please use another installation method.

### Scoop

Himalaya CLI can be installed with [Scoop](https://scoop.sh/):

```
scoop install himalaya
```

### Fedora Linux/CentOS/RHEL

Himalaya CLI can be installed on [Fedora Linux](https://fedoraproject.org/)/CentOS/RHEL via the [COPR](https://copr.fedorainfracloud.org/coprs/atim/himalaya/) repo:

```
dnf copr enable atim/himalaya
dnf install himalaya
```

### Nix

Himalaya CLI can be installed with [Nix](https://serokell.io/blog/what-is-nix):

```
nix-env -i himalaya
```

You can also use the git repository for a more up-to-date (but less stable) version:

```
nix-env -if https://github.com/pimalaya/himalaya/archive/master.tar.gz
```

*Or, from within the source tree checkout:*

```
nix-env -if .
```

If you have the [Flakes](https://nixos.wiki/wiki/Flakes) feature enabled:

```
nix profile install github:pimalaya/himalaya
```

*Or, from within the source tree checkout:*

```
nix profile install
```

*You can also run Himalaya directly without installing it:*

```
nix run github:pimalaya/himalaya
```

### Sources

```
git clone https://github.com/pimalaya/himalaya
cd himalaya
nix develop --command cargo build --release
```

*Binaries are available under the `target/release` folder.*

## Configuration

Just run `himalaya`. When no configuration file is found, the wizard prompts for an account name and email address, runs [provider discovery](https://github.com/pimalaya/io-discovery) (PACC → Thunderbird Autoconfiguration → RFC 6186 SRV), fills the IMAP/SMTP (or JMAP) prompts with the discovered defaults, and writes the result to disk.

Accounts can be (re)configured later with `himalaya account configure <name>`. The wizard skips discovery in this mode: it reuses the existing values as prompt defaults.

You can also write the configuration by hand:

- Copy the documented [`./config.sample.toml`](./config.sample.toml)
- Paste it into one of:
  - `$XDG_CONFIG_HOME/himalaya/config.toml`
  - `$HOME/.config/himalaya/config.toml`
  - `$HOME/.himalayarc`
- Comment or uncomment the options you want

…or pass `-c <PATH>` / set `HIMALAYA_CONFIG=<PATH>`. Multiple paths can be passed at once, separated by `:`; the first is the base and the rest are deep-merged on top.

## Usage

### Shared API

Backend-agnostic commands operate on the account's first configured backend, or the one selected with `-b/--backend`:

```
himalaya mailboxes list
himalaya envelopes list -m INBOX --page 2
himalaya envelopes list from alice and after 2026-01-01 order by date desc
himalaya flags add -m INBOX --flag seen 1:3,5
himalaya messages copy --from INBOX --to Archives 42
himalaya attachments download -m INBOX 42
```

When the `inbox` alias is configured under `[mailbox.alias]`, `-m/--mailbox` becomes optional: shared commands fall back to that id. With `[mailbox.alias] inbox = "INBOX"`, the calls above shorten to `envelopes list --page 2`, `flags add --flag seen 1:3,5`, etc.

`envelopes list` accepts a trailing search query covering `date`, `after`, `from`, `to`, `subject`, `body`, `flag` conditions (combined with `and`, `or`, `not`, grouped with parens) and a `order by date|from|to|subject [asc|desc]` sort chain. Date clauses target the `Date:` header (sent-at) on every backend.

Backend coverage:

- **IMAP**: full grammar via `SEARCH` (RFC 9051) + `SORT` (RFC 5256). `SENTON` / `SENTSINCE` keep date semantics anchored to the `Date:` header.
- **JMAP**: conjunctive filters only (`or` / `not` rejected; the JMAP wire model does not expose `FilterOperator` in `io-jmap` yet). Date clauses use an over-approximating `receivedAt` server filter plus a client-side `sentAt` post-filter so the sent-at rule is honored exactly.
- **Maildir**: full grammar except `body` (would require parsing every candidate message file; planned).

The shared surface is a strict least-common-denominator subset across IMAP, JMAP and Maildir. Operations that do not generalize (mailbox roles, attribute flags, JMAP-specific queries…) live under the protocol-specific subcommands.

### Protocol-specific APIs

Each backend exposes its full native API under its own subgroup:

```
himalaya imap mailboxes select INBOX
himalaya imap mailboxes status INBOX
himalaya imap mailboxes subscribe INBOX

himalaya jmap mailboxes query --role drafts
himalaya jmap identity get
himalaya jmap vacation get

himalaya maildir create Archives
himalaya maildir messages save -m ~/Mail/example/Archives < message.eml

himalaya smtp messages send < message.eml
```

The `-b/--backend` flag is only consumed by the shared commands; protocol subcommands always use their own backend.

### Composing messages

The built-in `messages compose` / `reply` / `forward` commands cover simple cases via CLI flags:

```
himalaya messages compose --from me@example.org --to you@example.org \
    --subject "Hello" --body "Hi!" --send
```

For richer composition (multipart MIME, MML directives, signing/encryption, editor-driven workflows…), wire a user-defined composer in `[message.composer.*]` and invoke it with the `-with` variants. For example, with [`mml`](https://github.com/pimalaya/mml):

```toml
[message.composer.mml]
command = "mml compose"
default = true
```

```
himalaya messages compose-with
himalaya messages reply-with -m INBOX 42 --send
himalaya messages forward-with -m INBOX 42 --send
himalaya messages mailto 'mailto:bob@example.org?subject=Hi&body=Hello'
```

`messages mailto <URI>` parses an RFC 6068 `mailto:` URI (recipient list in the path, `to` / `cc` / `bcc` / `subject` / `body` query parameters), builds a draft RFC 5322 skeleton with those headers pre-filled, then pipes it on stdin to the named (or default) composer for editing. The composer's output is routed through `--save` / `--send` like the other `-with` variants. Useful as a desktop `mailto:` handler.

### Reading messages

The built-in `messages read` command renders a message with himalaya's default formatter. For custom rendering, declare a reader in `[message.reader.*]` and call `read-with`:

```toml
[message.reader.mml]
command = "mml read"
default = true
```

```
himalaya messages read-with -m INBOX 42
```

### Re-using sessions

Each invocation opens a fresh TCP+TLS+SASL session by default. To amortize the handshake across many commands, pair himalaya with [`sirup`](https://github.com/pimalaya/sirup): `sirup` exposes a pre-authenticated IMAP/SMTP session over a Unix socket, and himalaya can point its `imap.server` / `smtp.server` at that socket.

## Interfaces

These interfaces are built at the top of Himalaya CLI to improve the User Experience:

- [pimalaya/himalaya-tui](https://github.com/pimalaya/himalaya-tui): official TUI (in active development)
- [pimalaya/himalaya-vim](https://github.com/pimalaya/himalaya-vim): Vim plugin
- [dantecatalfamo/himalaya-emacs](https://github.com/dantecatalfamo/himalaya-emacs): Emacs plugin
- [jns/himalaya](https://www.raycast.com/jns/himalaya): Raycast extension
- [openclaw/openclaw](https://github.com/openclaw/openclaw/blob/main/skills/himalaya/SKILL.md): OpenClaw SKILL
- [parisni/dfzf](https://github.com/parisni/dfzf): dfzf integration

## FAQ

<details>
  <summary>How different is it from aerc, mutt or alpine?</summary>

  Aerc, mutt and alpine can be categorized as Terminal User Interfaces (TUI). When the program is executed, your terminal is locked into an event loop and you interact with your emails using keybinds.

  Himalaya is a Command-Line Interface (CLI). There is no event loop: you interact with your emails using shell commands, in a stateless way.

  A dedicated TUI ([himalaya-tui](https://github.com/pimalaya/himalaya-tui)) is in active development on top of the same Pimalaya libraries.
</details>

<details>
  <summary>How are secrets resolved?</summary>

  Every `*.passwd` / `*.password` / `*.token` field accepts either a raw literal or a shell command that prints the secret on stdout. The raw form is convenient for testing but should not be used in production:

  ```toml
  imap.sasl.plain.passwd.raw = "***"
  imap.sasl.plain.passwd.command = "pass show example"
  imap.sasl.plain.passwd.command = ["pass", "show", "example"]
  ```

  Native keyring support was removed in v2. Use [pimalaya/mimosa](https://github.com/pimalaya/mimosa) (or `pass`, `secret-tool`, `gopass`…) as the `command`.
</details>

<details>
  <summary>How is OAuth 2.0 handled?</summary>

  v2 does not ship OAuth flows. Use [pimalaya/ortie](https://github.com/pimalaya/ortie) (or any other token broker) to obtain an access token, then plug it as a `command` returning the token on stdout. For JMAP, point `jmap.auth.bearer.token.command` at the broker; for IMAP/SMTP, route the bearer through a SASL mechanism that consumes a command-sourced password.
</details>

<details>
  <summary>How does the wizard discover IMAP/SMTP/JMAP configs?</summary>

  The wizard runs three discovery mechanisms in series on the email address domain; the first non-empty hit wins:

  1. **PACC** <sup>[draft-ietf-mailmaint-pacc-02](https://datatracker.ietf.org/doc/html/draft-ietf-mailmaint-pacc-02)</sup>: well-known JSON, digest-verified against the `_ua-auto-config` TXT record.
  2. **Thunderbird Autoconfiguration**: ISP main / well-known / ISPDB lookups, then MX-based retry, then the `mailconf=<URL>` TXT redirect.
  3. **RFC 6186 SRV**: `_imap._tcp`, `_imaps._tcp`, `_submission._tcp` lookups assembled into a single report.

  See [io-discovery](https://github.com/pimalaya/io-discovery) for the full chain.
</details>

<details>
  <summary>How to debug Himalaya CLI?</summary>

  Use `--log-level <level>` (alias `--log`) where `<level>` is one of `off`, `error`, `warn`, `info`, `debug`, `trace`:

  ```
  himalaya --log trace mailboxes list
  ```

  The `RUST_LOG` environment variable is consulted when `--log` is not passed, and supports per-target filters (see the [`env_logger` documentation](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)). `RUST_BACKTRACE=1` enables full error backtraces.

  Logs are written to `stderr`, so they can be redirected easily to a file:

  ```
  himalaya --log trace mailboxes list 2>/tmp/himalaya.log
  ```

  You can also send logs straight to a file via `--log-file <path>`:

  ```
  himalaya --log trace --log-file /tmp/himalaya.log mailboxes list
  ```
</details>

<details>
  <summary>How to disable color output?</summary>

  Set `NO_COLOR=1` in your environment.
</details>

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
- *2027 in preparation…*

If you appreciate the project, feel free to donate using one of the following providers:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
