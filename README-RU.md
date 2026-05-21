<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI для управления почтой</p>
  <p>
    <a href="https://github.com/pimalaya/himalaya/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/pimalaya/himalaya?color=success"/></a>
    <a href="https://repology.org/project/himalaya/versions"><img alt="Repology" src="https://img.shields.io/repology/repositories/himalaya?color=success"></a>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white"/></a>
    <a href="https://fosstodon.org/@pimalaya"><img alt="Mastodon" src="https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white"/></a>
  </p>
  <p>
    <strong>Языки / Languages:</strong>
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
> Этот README описывает Himalaya v2, который **ещё не выпущен**. Если вы используете v1 (`himalaya v1.2.0` или ранее), обратитесь к [README v1.2.0](https://github.com/pimalaya/himalaya/blob/v1.2.0/README.md). Руководство [MIGRATION.md](./MIGRATION.md) поможет пользователям v1 разобраться с несовместимыми изменениями.

## Содержание

- [Возможности](#возможности)
- [Установка](#установка)
  - [Готовый бинарник](#готовый-бинарник)
  - [Cargo](#cargo)
  - [Arch Linux](#arch-linux)
  - [Homebrew](#homebrew)
  - [Scoop](#scoop)
  - [Fedora Linux/CentOS/RHEL](#fedora-linuxcentosrhel)
  - [Nix](#nix)
  - [Исходники](#исходники)
- [Настройка](#настройка)
- [Использование](#использование)
  - [Общий API](#общий-api)
  - [API для конкретных протоколов](#api-для-конкретных-протоколов)
  - [Составление сообщений](#составление-сообщений)
  - [Чтение сообщений](#чтение-сообщений)
  - [Повторное использование сессий](#повторное-использование-сессий)
- [Интерфейсы](#интерфейсы)
- [FAQ](#faq)
- [Соцсети](#соцсети)
- [Спонсорство](#спонсорство)

## Возможности

- **Общий API**, сопоставляющий `mailboxes`, `envelopes`, `flags`, `messages` и `attachments` с активным бэкендом
- **API для конкретных протоколов**, раскрывающие полный набор возможностей каждого бэкенда (`himalaya imap/smtp/maildir/jmap…`)
- Поддержка **IMAP** <sup>[rfc9051](https://www.iana.org/go/rfc9051)</sup> (требуется feature `imap`)
- Поддержка **JMAP** <sup>[rfc8620](https://www.iana.org/go/rfc8620), [rfc8621](https://www.iana.org/go/rfc8621)</sup> (требуется feature `jmap`)
- Поддержка **Maildir** (требуется feature `maildir`)
- Бэкенд **SMTP** <sup>[rfc5321](https://www.iana.org/go/rfc5321)</sup> (требуется feature `smtp`)
- Поддержка **TLS**:
  - [native-tls](https://crates.io/crates/native-tls) (требуется feature `native-tls`)
  - [rustls](https://crates.io/crates/rustls):
    - Криптопровайдер AWS-LC (требуется feature `rustls-aws`)
    - Криптопровайдер Ring (требуется feature `rustls-ring`)
- Поддержка **SASL**: anonymous, login, plain, oauthbearer, xoauth2, scram-sha-256
- Мастер **обнаружения провайдера** на базе [io-discovery](https://github.com/pimalaya/io-discovery): Thunderbird Autoconfiguration, PACC и SRV-запросы RFC 6186
- **TOML**-конфигурация с поддержкой нескольких аккаунтов
- **JSON**-вывод через `--json`

*Himalaya CLI написан на [Rust](https://www.rust-lang.org/) и использует [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) для включения или отключения функций. Стандартные features можно найти в разделе `features` файла [`Cargo.toml`](./Cargo.toml#L18) или на [docs.rs](https://docs.rs/crate/himalaya/latest/features).*

## Установка

### Готовый бинарник

Himalaya CLI можно установить с помощью установщика `install.sh`:

*От root:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh
```

*От обычного пользователя:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
```

Эти команды устанавливают последний бинарник из раздела [releases](https://github.com/pimalaya/himalaya/releases) на GitHub.

Если вам нужна более свежая версия, чем последний релиз, посмотрите workflow [releases](https://github.com/pimalaya/himalaya/actions/workflows/releases.yml) на GitHub и найдите раздел *Artifacts*. Там будет готовый бинарник для вашей ОС. Эти бинарники собираются из ветки `master`.

*Такие бинарники собираются со стандартными cargo features. Если нужны дополнительные features, используйте другой способ установки.*

### Cargo

Himalaya CLI можно установить через [cargo](https://doc.rust-lang.org/cargo/):

```
cargo install himalaya --locked
```

Только с поддержкой IMAP:

```
cargo install himalaya --locked --no-default-features --features imap
```

Также можно использовать git-репозиторий для более свежей (но менее стабильной) версии:

```
cargo install --locked --git https://github.com/pimalaya/himalaya.git
```

### Arch Linux

Himalaya CLI можно установить на [Arch Linux](https://archlinux.org/) из community-репозитория:

```
pacman -S himalaya
```

или из [AUR](https://aur.archlinux.org/):

```
git clone https://aur.archlinux.org/himalaya-git.git
cd himalaya-git
makepkg -isc
```

Если вы используете [yay](https://github.com/Jguer/yay), это ещё проще:

```
yay -S himalaya-git
```

### Homebrew

Himalaya CLI можно установить через [Homebrew](https://brew.sh/):

```
brew install himalaya
```

Примечание: cargo features несовместимы с brew. Если нужен другой набор features, используйте другой способ установки.

### Scoop

Himalaya CLI можно установить через [Scoop](https://scoop.sh/):

```
scoop install himalaya
```

### Fedora Linux/CentOS/RHEL

Himalaya CLI можно установить на [Fedora Linux](https://fedoraproject.org/)/CentOS/RHEL через репозиторий [COPR](https://copr.fedorainfracloud.org/coprs/atim/himalaya/):

```
dnf copr enable atim/himalaya
dnf install himalaya
```

### Nix

Himalaya CLI можно установить через [Nix](https://serokell.io/blog/what-is-nix):

```
nix-env -i himalaya
```

Также можно использовать git-репозиторий для более свежей (но менее стабильной) версии:

```
nix-env -if https://github.com/pimalaya/himalaya/archive/master.tar.gz
```

*Или из checkout исходного дерева:*

```
nix-env -if .
```

Если у вас включена функция [Flakes](https://nixos.wiki/wiki/Flakes):

```
nix profile install github:pimalaya/himalaya
```

*Или из checkout исходного дерева:*

```
nix profile install
```

*Также можно запускать Himalaya напрямую без установки:*

```
nix run github:pimalaya/himalaya
```

### Исходники

```
git clone https://github.com/pimalaya/himalaya
cd himalaya
nix develop --command cargo build --release
```

*Бинарники доступны в папке `target/release`.*

## Настройка

Просто запустите `himalaya`. Если конфигурационный файл не найден, мастер запросит имя аккаунта и адрес электронной почты, выполнит [обнаружение провайдера](https://github.com/pimalaya/io-discovery) (PACC → Thunderbird Autoconfiguration → RFC 6186 SRV), заполнит запросы IMAP/SMTP (или JMAP) обнаруженными значениями по умолчанию и запишет результат на диск.

Аккаунты можно (пере)настроить позже командой `himalaya account configure <name>`. В этом режиме мастер пропускает обнаружение: он повторно использует существующие значения как значения по умолчанию для запросов.

Конфигурацию также можно написать вручную:

- Скопируйте документированный [./config.sample.toml](./config.sample.toml)
- Вставьте в один из путей:
  - `$XDG_CONFIG_HOME/himalaya/config.toml`
  - `$HOME/.config/himalaya/config.toml`
  - `$HOME/.himalayarc`
- Закомментируйте или раскомментируйте нужные опции

…или передайте `-c <PATH>` / установите `HIMALAYA_CONFIG=<PATH>`. Можно передать несколько путей сразу, разделённых `:`; первый — базовый, остальные глубоко объединяются поверх.

## Использование

### Общий API

Команды, не зависящие от бэкенда, работают с первым настроенным бэкендом аккаунта или с выбранным через `-b/--backend`:

```
himalaya mailboxes list
himalaya envelopes list -m INBOX --page 2
himalaya envelopes search from alice and after 2026-01-01 order by date desc
himalaya flags add -m INBOX --flag seen 1:3,5
himalaya messages copy --from INBOX --to Archives 42
himalaya attachments download -m INBOX 42
```

Когда alias `inbox` настроен в `[mailbox.alias]`, `-m/--mailbox` становится необязательным: общие команды используют этот id по умолчанию. При `[mailbox.alias] inbox = "INBOX"` вызовы выше сокращаются до `envelopes list --page 2`, `flags add --flag seen 1:3,5` и т.д.

`envelopes list` — простая пагинация, отсортированная по дате по убыванию. Для фильтрации или сортировки используйте `envelopes search` с завершающим запросом, охватывающим условия `date`, `after`, `from`, `to`, `subject`, `body`, `flag` (объединённые через `and`, `or`, `not`, с группировкой в скобках) и цепочку сортировки `order by date|from|to|subject [asc|desc]`. Условия по дате относятся к заголовку `Date:` (время отправки) на всех бэкендах.

Общая поверхность — строгое подмножество наименьшего общего знаменателя для IMAP, JMAP и Maildir. Операции, которые не обобщаются (роли почтовых ящиков, атрибутные флаги, JMAP-специфичные запросы…), находятся в подкомандах для конкретных протоколов.

### API для конкретных протоколов

Каждый бэкенд раскрывает свой полный нативный API в собственной подгруппе:

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

Флаг `-b/--backend` используется только общими командами; подкоманды протоколов всегда используют свой собственный бэкенд.

### Составление сообщений

Встроенные команды `messages compose` / `reply` / `forward` покрывают простые случаи через флаги CLI:

```
himalaya messages compose --from me@example.org --to you@example.org \
    --subject "Hello" --body "Hi!" --send
```

Для более богатого составления (multipart MIME, директивы MML, подпись/шифрование, рабочие процессы через редактор…) настройте пользовательский композер в `[message.composer.*]` и вызывайте его через варианты `-with`. Например, с [`mml`](https://github.com/pimalaya/mml):

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

`messages mailto <URI>` разбирает URI RFC 6068 `mailto:` (список получателей в пути, query-параметры `to` / `cc` / `bcc` / `subject` / `body`), создаёт черновой каркас RFC 5322 с предзаполненными заголовками и передаёт его на stdin именованному (или стандартному) композеру для редактирования. Вывод композера направляется через `--save` / `--send`, как и у других вариантов `-with`. Полезно как обработчик `mailto:` на рабочем столе.

### Чтение сообщений

Встроенная команда `messages read` отображает сообщение с форматтером himalaya по умолчанию. Для пользовательского отображения объявите reader в `[message.reader.*]` и вызовите `read-with`:

```toml
[message.reader.mml]
command = "mml read"
default = true
```

```
himalaya messages read-with -m INBOX 42
```

### Повторное использование сессий

Каждый вызов по умолчанию открывает новую сессию TCP+TLS+SASL. Чтобы амортизировать рукопожатие на множестве команд, используйте himalaya вместе с [`sirup`](https://github.com/pimalaya/sirup): `sirup` предоставляет предварительно аутентифицированную сессию IMAP/SMTP через Unix-сокет, а himalaya может направить свой `imap.server` / `smtp.server` на этот сокет.

## Интерфейсы

Эти интерфейсы построены поверх Himalaya CLI для улучшения пользовательского опыта:

- [pimalaya/himalaya-tui](https://github.com/pimalaya/himalaya-tui): официальный TUI (в активной разработке)
- [pimalaya/himalaya-vim](https://github.com/pimalaya/himalaya-vim): плагин Vim
- [dantecatalfamo/himalaya-emacs](https://github.com/dantecatalfamo/himalaya-emacs): плагин Emacs
- [jns/himalaya](https://www.raycast.com/jns/himalaya): расширение Raycast
- [openclaw/openclaw](https://github.com/openclaw/openclaw/blob/main/skills/himalaya/SKILL.md): SKILL OpenClaw
- [parisni/dfzf](https://github.com/parisni/dfzf): интеграция dfzf

## FAQ

<details>
  <summary>Чем он отличается от aerc, mutt или alpine?</summary>

  Aerc, mutt и alpine можно отнести к терминальным пользовательским интерфейсам (TUI). При запуске программы терминал блокируется в цикле событий, и вы взаимодействуете с почтой с помощью горячих клавиш.

  Himalaya — это интерфейс командной строки (CLI). Цикла событий нет: вы работаете с почтой через shell-команды, без сохранения состояния.

  Специализированный TUI ([himalaya-tui](https://github.com/pimalaya/himalaya-tui)) активно разрабатывается на тех же библиотеках Pimalaya.
</details>

<details>
  <summary>Как разрешаются секреты?</summary>

  Каждое поле `*.passwd` / `*.password` / `*.token` принимает либо сырой литерал, либо shell-команду, выводящую секрет в stdout. Сырой вариант удобен для тестирования, но не должен использоваться в production:

  ```toml
  imap.sasl.plain.passwd.raw = "***"
  imap.sasl.plain.passwd.command = "pass show example"
  imap.sasl.plain.passwd.command = ["pass", "show", "example"]
  ```

  Нативная поддержка keyring была удалена в v2. Используйте [pimalaya/mimosa](https://github.com/pimalaya/mimosa) (или `pass`, `secret-tool`, `gopass`…) в качестве `command`.
</details>

<details>
  <summary>Как обрабатывается OAuth 2.0?</summary>

  v2 не включает OAuth-потоки. Используйте [pimalaya/ortie](https://github.com/pimalaya/ortie) (или любой другой брокер токенов), чтобы получить access token, и подключите его как `command`, возвращающий токен в stdout. Для JMAP укажите `jmap.auth.bearer.token.command` на брокер; для IMAP/SMTP направьте bearer через SASL-механизм, потребляющий пароль из command.
</details>

<details>
  <summary>Как мастер обнаруживает конфигурации IMAP/SMTP/JMAP?</summary>

  Мастер последовательно запускает три механизма обнаружения для домена адреса электронной почты; побеждает первый непустой результат:

  1. **PACC** <sup>[draft-ietf-mailmaint-pacc-02](https://datatracker.ietf.org/doc/html/draft-ietf-mailmaint-pacc-02)</sup>: well-known JSON, проверенный digest против TXT-записи `_ua-auto-config`.
  2. **Thunderbird Autoconfiguration**: запросы ISP main / well-known / ISPDB, затем повторная попытка по MX, затем TXT-перенаправление `mailconf=<URL>`.
  3. **RFC 6186 SRV**: запросы `_imap._tcp`, `_imaps._tcp`, `_submission._tcp`, собранные в один отчёт.

  Полную цепочку см. в [io-discovery](https://github.com/pimalaya/io-discovery).
</details>

<details>
  <summary>Как отладить Himalaya CLI?</summary>

  Используйте `--log-level <level>` (алиас `--log`), где `<level>` — одно из `off`, `error`, `warn`, `info`, `debug`, `trace`:

  ```
  himalaya --log trace mailboxes list
  ```

  Переменная окружения `RUST_LOG` учитывается, когда `--log` не передан, и поддерживает фильтры по целям (см. [документацию `env_logger`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)). `RUST_BACKTRACE=1` включает полные backtrace ошибок.

  Логи пишутся в `stderr`, поэтому их легко перенаправить в файл:

  ```
  himalaya --log trace mailboxes list 2>/tmp/himalaya.log
  ```

  Также можно отправлять логи прямо в файл через `--log-file <path>`:

  ```
  himalaya --log trace --log-file /tmp/himalaya.log mailboxes list
  ```
</details>

<details>
  <summary>Как отключить цветной вывод?</summary>

  Установите `NO_COLOR=1` в окружении.
</details>

## Соцсети

- Чат в [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- Новости в [Mastodon](https://fosstodon.org/@pimalaya) или [RSS](https://fosstodon.org/@pimalaya.rss)
- Почта: [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Спонсорство

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Особая благодарность [фонду NLnet](https://nlnet.nl/) и [Европейской комиссии](https://www.ngi.eu/), которые финансово поддерживают проект уже много лет:

- 2022 → 2023: [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024: [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026: [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- *2027 в подготовке…*

Если вам нравится проект, вы можете поддержать его через одного из следующих провайдеров:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
