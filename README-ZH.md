<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI 管理邮件</p>
  <p>
    <a href="https://github.com/pimalaya/himalaya/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/pimalaya/himalaya?color=success"/></a>
    <a href="https://repology.org/project/himalaya/versions"><img alt="Repology" src="https://img.shields.io/repology/repositories/himalaya?color=success"></a>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white"/></a>
    <a href="https://fosstodon.org/@pimalaya"><img alt="Mastodon" src="https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white"/></a>
  </p>
  <p>
    <strong>语言 / Languages:</strong>
    <a href="README.md">English</a> ·
    <a href="README-ZH.md">中文</a> ·
    <a href="README-ES.md">Español</a> ·
    <a href="README-FR.md">Français</a> ·
    <a href="README-PT.md">Português</a> ·
    <a href="README-RU.md">Русский</a> ·
    <a href="README-DE.md">Deutsch</a>
  </p>
</div>

```
himalaya envelopes list --account posteo -m Archives.FOSS --page 2
```

![screenshot](./screenshot.jpeg)

> [!IMPORTANT]
> 本 README 记录的是尚未发布的 Himalaya v2。若你正在使用 v1（`himalaya v1.2.0` 或更早版本），请参阅 [v1.2.0 README](https://github.com/pimalaya/himalaya/blob/v1.2.0/README.md)。[MIGRATION.md](./MIGRATION.md) 指南可帮助 v1 用户了解破坏性变更。

## 目录

- [功能](#features)
- [安装](#installation)
  - [预编译二进制](#pre-built-binary)
  - [Cargo](#cargo)
  - [Arch Linux](#arch-linux)
  - [Homebrew](#homebrew)
  - [Scoop](#scoop)
  - [Fedora Linux/CentOS/RHEL](#fedora-linuxcentosrhel)
  - [Nix](#nix)
  - [源码](#sources)
- [配置](#configuration)
- [用法](#usage)
  - [共享 API](#shared-api)
  - [协议专用 API](#protocol-specific-apis)
  - [撰写邮件](#composing-messages)
  - [阅读邮件](#reading-messages)
  - [复用会话](#re-using-sessions)
- [界面](#interfaces)
- [常见问题](#faq)
- [社区](#social)
- [赞助](#sponsoring)

## 功能

- **共享 API**，将 `mailboxes`、`envelopes`、`flags`、`messages` 和 `attachments` 映射到当前后端
- **协议专用 API**，暴露各后端的完整能力（`himalaya imap/smtp/maildir/jmap…`）
- **IMAP** 支持 <sup>[rfc9051](https://www.iana.org/go/rfc9051)</sup>（需要 `imap` feature）
- **JMAP** 支持 <sup>[rfc8620](https://www.iana.org/go/rfc8620), [rfc8621](https://www.iana.org/go/rfc8621)</sup>（需要 `jmap` feature）
- **Maildir** 支持（需要 `maildir` feature）
- **SMTP** 后端 <sup>[rfc5321](https://www.iana.org/go/rfc5321)</sup>（需要 `smtp` feature）
- **TLS** 支持：
  - [native-tls](https://crates.io/crates/native-tls)（需要 `native-tls` feature）
  - [rustls](https://crates.io/crates/rustls)：
    - AWS-LC 加密提供方（需要 `rustls-aws` feature）
    - Ring 加密提供方（需要 `rustls-ring` feature）
- **SASL** 支持：anonymous、login、plain、oauthbearer、xoauth2、scram-sha-256
- 由 [io-discovery](https://github.com/pimalaya/io-discovery) 驱动的**服务商发现**向导：Thunderbird Autoconfiguration、PACC 与 RFC 6186 SRV 查询
- 支持多账户的 **TOML** 配置
- 通过 `--json` 输出 **JSON**

*Himalaya CLI 使用 [Rust](https://www.rust-lang.org/) 编写，并依赖 [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) 启用或禁用功能。默认 feature 见 [`Cargo.toml`](./Cargo.toml#L18) 的 `features` 节，或 [docs.rs](https://docs.rs/crate/himalaya/latest/features)。*

## 安装

### 预编译二进制

可使用 `install.sh` 安装 Himalaya CLI：

*以 root 运行：*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh
```

*以普通用户运行：*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
```

上述命令会从 GitHub [releases](https://github.com/pimalaya/himalaya/releases) 安装最新二进制。

若需要比最新 release 更新的版本，可查看 [releases](https://github.com/pimalaya/himalaya/actions/workflows/releases.yml) GitHub 工作流中的 *Artifacts* 部分，会有匹配你操作系统的预编译二进制。这些二进制由 `master` 分支构建。

*此类二进制使用默认 cargo features 构建。若需要更多 feature，请改用其他安装方式。*

### Cargo

可使用 [cargo](https://doc.rust-lang.org/cargo/) 安装 Himalaya CLI：

```
cargo install himalaya --locked
```

仅启用 IMAP 支持：

```
cargo install himalaya --locked --no-default-features --features imap
```

也可使用 git 仓库获取更新（但稳定性较低）的版本：

```
cargo install --locked --git https://github.com/pimalaya/himalaya.git
```

### Arch Linux

在 [Arch Linux](https://archlinux.org/) 上可通过社区仓库安装：

```
pacman -S himalaya
```

或通过 [用户仓库](https://aur.archlinux.org/)：

```
git clone https://aur.archlinux.org/himalaya-git.git
cd himalaya-git
makepkg -isc
```

若使用 [yay](https://github.com/Jguer/yay)，更简单：

```
yay -S himalaya-git
```

### Homebrew

可使用 [Homebrew](https://brew.sh/) 安装：

```
brew install himalaya
```

注意：brew 与 cargo features 不兼容。若需要不同的 feature 组合，请改用其他安装方式。

### Scoop

可使用 [Scoop](https://scoop.sh/) 安装：

```
scoop install himalaya
```

### Fedora Linux/CentOS/RHEL

在 [Fedora Linux](https://fedoraproject.org/)/CentOS/RHEL 上可通过 [COPR](https://copr.fedorainfracloud.org/coprs/atim/himalaya/) 仓库安装：

```
dnf copr enable atim/himalaya
dnf install himalaya
```

### Nix

可使用 [Nix](https://serokell.io/blog/what-is-nix) 安装：

```
nix-env -i himalaya
```

也可使用 git 仓库获取更新（但稳定性较低）的版本：

```
nix-env -if https://github.com/pimalaya/himalaya/archive/master.tar.gz
```

*或在源码树检出目录内：*

```
nix-env -if .
```

若已启用 [Flakes](https://nixos.wiki/wiki/Flakes)：

```
nix profile install github:pimalaya/himalaya
```

*或在源码树检出目录内：*

```
nix profile install
```

*也可不安装直接运行 Himalaya：*

```
nix run github:pimalaya/himalaya
```

### 源码

```
git clone https://github.com/pimalaya/himalaya
cd himalaya
nix develop --command cargo build --release
```

*二进制位于 `target/release` 目录。*

## 配置

直接运行 `himalaya`。未找到配置文件时，向导会提示输入账户名和邮箱地址，执行 [服务商发现](https://github.com/pimalaya/io-discovery)（PACC → Thunderbird Autoconfiguration → RFC 6186 SRV），用发现到的默认值填充 IMAP/SMTP（或 JMAP）提示，并将结果写入磁盘。

之后可用 `himalaya account configure <name>` 重新配置账户。此模式下向导跳过发现：以现有值作为提示的默认值。

也可手动编写配置：

- 复制文档化的 [./config.sample.toml](./config.sample.toml)
- 粘贴到以下之一：
  - `$XDG_CONFIG_HOME/himalaya/config.toml`
  - `$HOME/.config/himalaya/config.toml`
  - `$HOME/.himalayarc`
- 注释或取消注释你需要的选项

…或通过 `-c <PATH>` / 设置 `HIMALAYA_CONFIG=<PATH>` 指定。可一次传入多个路径，用 `:` 分隔；第一个为基础配置，其余在其上深度合并。

## 用法

### 共享 API

与后端无关的命令作用于账户配置的第一个后端，或通过 `-b/--backend` 选择的后端：

```
himalaya mailboxes list
himalaya envelopes list -m INBOX --page 2
himalaya envelopes search from alice and after 2026-01-01 order by date desc
himalaya flags add -m INBOX --flag seen 1:3,5
himalaya messages copy --from INBOX --to Archives 42
himalaya attachments download -m INBOX 42
```

在 `[mailbox.alias]` 下配置了 `inbox` 别名后，`-m/--mailbox` 变为可选：共享命令会回退到该 id。例如 `[mailbox.alias] inbox = "INBOX"` 时，上述调用可简化为 `envelopes list --page 2`、`flags add --flag seen 1:3,5` 等。

`envelopes list` 为按日期降序的普通分页。要筛选或排序，请使用 `envelopes search` 及尾部查询，支持 `date`、`after`、`from`、`to`、`subject`、`body`、`flag` 条件（用 `and`、`or`、`not` 组合，括号分组）以及 `order by date|from|to|subject [asc|desc]` 排序链。日期子句针对各后端的 `Date:` 头（发送时间）。

共享接口是 IMAP、JMAP 与 Maildir 之间的严格最小公倍数子集。无法泛化的操作（邮箱角色、属性标志、JMAP 专用查询等）位于协议专用子命令下。

### 协议专用 API

每个后端在其子组下暴露完整原生 API：

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

`-b/--backend` 标志仅由共享命令消费；协议子命令始终使用各自的后端。

### 撰写邮件

内置的 `messages compose` / `reply` / `forward` 命令通过 CLI 标志覆盖简单场景：

```
himalaya messages compose --from me@example.org --to you@example.org \
    --subject "Hello" --body "Hi!" --send
```

更丰富的撰写（多部分 MIME、MML 指令、签名/加密、编辑器驱动工作流等）可在 `[message.composer.*]` 中配置用户定义的 composer，并通过 `-with` 变体调用。例如配合 [`mml`](https://github.com/pimalaya/mml)：

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

`messages mailto <URI>` 解析 RFC 6068 `mailto:` URI（路径中的收件人列表，`to` / `cc` / `bcc` / `subject` / `body` 查询参数），构建预填这些头的 RFC 5322 草稿骨架，再通过 stdin 传给指定（或默认）composer 编辑。composer 输出经 `--save` / `--send` 路由，与其他 `-with` 变体相同。适合作为桌面 `mailto:` 处理程序。

### 阅读邮件

内置 `messages read` 使用 himalaya 默认格式化器渲染邮件。自定义渲染可在 `[message.reader.*]` 中声明 reader，并调用 `read-with`：

```toml
[message.reader.mml]
command = "mml read"
default = true
```

```
himalaya messages read-with -m INBOX 42
```

### 复用会话

默认每次调用都会建立新的 TCP+TLS+SASL 会话。要在多条命令间摊销握手开销，可将 himalaya 与 [`sirup`](https://github.com/pimalaya/sirup) 配合：`sirup` 通过 Unix 套接字暴露已认证的 IMAP/SMTP 会话，himalaya 可将 `imap.server` / `smtp.server` 指向该套接字。

## 界面

以下界面基于 Himalaya CLI 构建，以改善用户体验：

- [pimalaya/himalaya-tui](https://github.com/pimalaya/himalaya-tui)：官方 TUI（积极开发中）
- [pimalaya/himalaya-vim](https://github.com/pimalaya/himalaya-vim)：Vim 插件
- [dantecatalfamo/himalaya-emacs](https://github.com/dantecatalfamo/himalaya-emacs)：Emacs 插件
- [jns/himalaya](https://www.raycast.com/jns/himalaya)：Raycast 扩展
- [openclaw/openclaw](https://github.com/openclaw/openclaw/blob/main/skills/himalaya/SKILL.md)：OpenClaw SKILL
- [parisni/dfzf](https://github.com/parisni/dfzf)：dfzf 集成

## 常见问题

<details>
  <summary>与 aerc、mutt 或 alpine 有何不同？</summary>

  aerc、mutt 和 alpine 可归类为终端用户界面（TUI）。程序运行后，终端进入事件循环，通过按键与邮件交互。

  Himalaya 是命令行界面（CLI）。没有事件循环：通过 shell 命令以无状态方式与邮件交互。

  基于同一 Pimalaya 库的专用 TUI（[himalaya-tui](https://github.com/pimalaya/himalaya-tui)）正在积极开发中。
</details>

<details>
  <summary>密钥如何解析？</summary>

  每个 `*.passwd` / `*.password` / `*.token` 字段可接受原始字面量，或向 stdout 输出密钥的 shell 命令。原始形式便于测试，不应在生产环境使用：

  ```toml
  imap.sasl.plain.passwd.raw = "***"
  imap.sasl.plain.passwd.command = "pass show example"
  imap.sasl.plain.passwd.command = ["pass", "show", "example"]
  ```

  v2 已移除原生钥匙串支持。请将 [pimalaya/mimosa](https://github.com/pimalaya/mimosa)（或 `pass`、`secret-tool`、`gopass` 等）用作 `command`。
</details>

<details>
  <summary>OAuth 2.0 如何处理？</summary>

  v2 不内置 OAuth 流程。使用 [pimalaya/ortie](https://github.com/pimalaya/ortie)（或其他令牌代理）获取访问令牌，再将其作为向 stdout 返回令牌的 `command` 接入。JMAP 将 `jmap.auth.bearer.token.command` 指向代理；IMAP/SMTP 通过消费 command 来源密码的 SASL 机制传递 bearer。
</details>

<details>
  <summary>向导如何发现 IMAP/SMTP/JMAP 配置？</summary>

  向导在邮箱域名上依次运行三种发现机制；第一个非空结果生效：

  1. **PACC** <sup>[draft-ietf-mailmaint-pacc-02](https://datatracker.ietf.org/doc/html/draft-ietf-mailmaint-pacc-02)</sup>：well-known JSON，对照 `_ua-auto-config` TXT 记录进行摘要校验。
  2. **Thunderbird Autoconfiguration**：ISP main / well-known / ISPDB 查询，然后基于 MX 重试，再处理 `mailconf=<URL>` TXT 重定向。
  3. **RFC 6186 SRV**：查询 `_imap._tcp`、`_imaps._tcp`、`_submission._tcp` 并组装为单一报告。

  完整链路见 [io-discovery](https://github.com/pimalaya/io-discovery)。
</details>

<details>
  <summary>如何调试 Himalaya CLI？</summary>

  使用 `--log-level <level>`（别名 `--log`），`<level>` 为 `off`、`error`、`warn`、`info`、`debug`、`trace` 之一：

  ```
  himalaya --log trace mailboxes list
  ```

  未传递 `--log` 时会读取 `RUST_LOG` 环境变量，支持按目标筛选（见 [`env_logger` 文档](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)）。`RUST_BACKTRACE=1` 可启用完整错误回溯。

  日志写入 `stderr`，便于重定向到文件：

  ```
  himalaya --log trace mailboxes list 2>/tmp/himalaya.log
  ```

  也可通过 `--log-file <path>` 直接写入文件：

  ```
  himalaya --log trace --log-file /tmp/himalaya.log mailboxes list
  ```
</details>

<details>
  <summary>如何禁用彩色输出？</summary>

  在环境中设置 `NO_COLOR=1`。
</details>

## 社区

- 在 [Matrix](https://matrix.to/#/#pimalaya:matrix.org) 聊天
- 在 [Mastodon](https://fosstodon.org/@pimalaya) 或 [RSS](https://fosstodon.org/@pimalaya.rss) 获取动态
- 邮件联系 [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## 赞助

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

特别感谢多年来为项目提供资金支持的 [NLnet 基金会](https://nlnet.nl/)与[欧盟委员会](https://www.ngi.eu/)：

- 2022 → 2023：[NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024：[NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026：[NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- *2027 筹备中…*

若你欣赏本项目，欢迎通过以下渠道捐赠：

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
