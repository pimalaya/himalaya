<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI zur E-Mail-Verwaltung</p>
  <p>
    <a href="https://github.com/pimalaya/himalaya/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/pimalaya/himalaya?color=success"/></a>
    <a href="https://repology.org/project/himalaya/versions"><img alt="Repology" src="https://img.shields.io/repology/repositories/himalaya?color=success"></a>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white"/></a>
    <a href="https://fosstodon.org/@pimalaya"><img alt="Mastodon" src="https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white"/></a>
  </p>
  <p>
    <strong>Sprachen / Languages:</strong>
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
> Dieses README dokumentiert Himalaya v2, das **noch nicht veröffentlicht** ist. Wenn Sie v1 verwenden (`himalaya v1.2.0` oder früher), lesen Sie stattdessen das [README v1.2.0](https://github.com/pimalaya/himalaya/blob/v1.2.0/README.md). Der Leitfaden [MIGRATION.md](./MIGRATION.md) führt v1-Nutzer durch die Breaking Changes.

## Inhaltsverzeichnis

- [Funktionen](#funktionen)
- [Installation](#installation)
  - [Vorkompiliertes Binary](#vorkompiliertes-binary)
  - [Cargo](#cargo)
  - [Arch Linux](#arch-linux)
  - [Homebrew](#homebrew)
  - [Scoop](#scoop)
  - [Fedora Linux/CentOS/RHEL](#fedora-linuxcentosrhel)
  - [Nix](#nix)
  - [Quellcode](#quellcode)
- [Konfiguration](#konfiguration)
- [Verwendung](#verwendung)
  - [Gemeinsame API](#gemeinsame-api)
  - [Protokollspezifische APIs](#protokollspezifische-apis)
  - [Nachrichten verfassen](#nachrichten-verfassen)
  - [Nachrichten lesen](#nachrichten-lesen)
  - [Sitzungen wiederverwenden](#sitzungen-wiederverwenden)
- [Schnittstellen](#schnittstellen)
- [FAQ](#faq)
- [Social Media](#social-media)
- [Sponsoring](#sponsoring)

## Funktionen

- **Gemeinsame API**, die `mailboxes`, `envelopes`, `flags`, `messages` und `attachments` dem aktiven Backend zuordnet
- **Protokollspezifische APIs**, die die vollständige Oberfläche jedes Backends bereitstellen (`himalaya imap/smtp/maildir/jmap…`)
- **IMAP**-Unterstützung <sup>[rfc9051](https://www.iana.org/go/rfc9051)</sup> (erfordert das Feature `imap`)
- **JMAP**-Unterstützung <sup>[rfc8620](https://www.iana.org/go/rfc8620), [rfc8621](https://www.iana.org/go/rfc8621)</sup> (erfordert das Feature `jmap`)
- **Maildir**-Unterstützung (erfordert das Feature `maildir`)
- **SMTP**-Backend <sup>[rfc5321](https://www.iana.org/go/rfc5321)</sup> (erfordert das Feature `smtp`)
- **TLS**-Unterstützung:
  - [native-tls](https://crates.io/crates/native-tls) (erfordert das Feature `native-tls`)
  - [rustls](https://crates.io/crates/rustls):
    - AWS-LC-Kryptoprovider (erfordert das Feature `rustls-aws`)
    - Ring-Kryptoprovider (erfordert das Feature `rustls-ring`)
- **SASL**-Unterstützung: anonymous, login, plain, oauthbearer, xoauth2, scram-sha-256
- **Provider-Erkennung**-Assistent auf Basis von [io-discovery](https://github.com/pimalaya/io-discovery): Thunderbird Autoconfiguration, PACC und RFC-6186-SRV-Lookups
- **TOML**-Konfiguration mit Multi-Account-Unterstützung
- **JSON**-Ausgabe über `--json`

*Himalaya CLI ist in [Rust](https://www.rust-lang.org/) geschrieben und nutzt [cargo features](https://doc.rust-lang.org/cargo/reference/features.html), um Funktionen zu aktivieren oder zu deaktivieren. Standard-Features finden Sie im Abschnitt `features` der [`Cargo.toml`](./Cargo.toml#L18) oder auf [docs.rs](https://docs.rs/crate/himalaya/latest/features).*

## Installation

### Vorkompiliertes Binary

Himalaya CLI kann mit dem Installer `install.sh` installiert werden:

*Als root:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh
```

*Als normaler Benutzer:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
```

Diese Befehle installieren das neueste Binary aus dem GitHub-Abschnitt [releases](https://github.com/pimalaya/himalaya/releases).

Wenn Sie eine aktuellere Version als das neueste Release benötigen, schauen Sie im GitHub-Workflow [releases](https://github.com/pimalaya/himalaya/actions/workflows/releases.yml) nach dem Abschnitt *Artifacts*. Dort finden Sie ein vorkompiliertes Binary für Ihr Betriebssystem. Diese Binaries werden aus dem Branch `master` gebaut.

*Solche Binaries werden mit den Standard-cargo-features gebaut. Wenn Sie mehr Features benötigen, verwenden Sie bitte eine andere Installationsmethode.*

### Cargo

Himalaya CLI kann mit [cargo](https://doc.rust-lang.org/cargo/) installiert werden:

```
cargo install himalaya --locked
```

Nur mit IMAP-Unterstützung:

```
cargo install himalaya --locked --no-default-features --features imap
```

Sie können auch das Git-Repository für eine aktuellere (aber weniger stabile) Version verwenden:

```
cargo install --locked --git https://github.com/pimalaya/himalaya.git
```

### Arch Linux

Himalaya CLI kann auf [Arch Linux](https://archlinux.org/) über das Community-Repository installiert werden:

```
pacman -S himalaya
```

oder über das [User Repository](https://aur.archlinux.org/):

```
git clone https://aur.archlinux.org/himalaya-git.git
cd himalaya-git
makepkg -isc
```

Wenn Sie [yay](https://github.com/Jguer/yay) verwenden, ist es noch einfacher:

```
yay -S himalaya-git
```

### Homebrew

Himalaya CLI kann mit [Homebrew](https://brew.sh/) installiert werden:

```
brew install himalaya
```

Hinweis: cargo features sind nicht mit brew kompatibel. Wenn Sie ein anderes Feature-Set benötigen, verwenden Sie bitte eine andere Installationsmethode.

### Scoop

Himalaya CLI kann mit [Scoop](https://scoop.sh/) installiert werden:

```
scoop install himalaya
```

### Fedora Linux/CentOS/RHEL

Himalaya CLI kann auf [Fedora Linux](https://fedoraproject.org/)/CentOS/RHEL über das [COPR](https://copr.fedorainfracloud.org/coprs/atim/himalaya/)-Repository installiert werden:

```
dnf copr enable atim/himalaya
dnf install himalaya
```

### Nix

Himalaya CLI kann mit [Nix](https://serokell.io/blog/what-is-nix) installiert werden:

```
nix-env -i himalaya
```

Sie können auch das Git-Repository für eine aktuellere (aber weniger stabile) Version verwenden:

```
nix-env -if https://github.com/pimalaya/himalaya/archive/master.tar.gz
```

*Oder aus dem Checkout des Quellbaums:*

```
nix-env -if .
```

Wenn Sie die [Flakes](https://nixos.wiki/wiki/Flakes)-Funktion aktiviert haben:

```
nix profile install github:pimalaya/himalaya
```

*Oder aus dem Checkout des Quellbaums:*

```
nix profile install
```

*Sie können Himalaya auch direkt ohne Installation ausführen:*

```
nix run github:pimalaya/himalaya
```

### Quellcode

```
git clone https://github.com/pimalaya/himalaya
cd himalaya
nix develop --command cargo build --release
```

*Binaries sind im Ordner `target/release` verfügbar.*

## Konfiguration

Führen Sie einfach `himalaya` aus. Wenn keine Konfigurationsdatei gefunden wird, fragt der Assistent nach Kontoname und E-Mail-Adresse, führt [Provider-Erkennung](https://github.com/pimalaya/io-discovery) durch (PACC → Thunderbird Autoconfiguration → RFC 6186 SRV), füllt die IMAP/SMTP- (oder JMAP-)Eingaben mit den erkannten Standardwerten und schreibt das Ergebnis auf die Festplatte.

Konten können später mit `himalaya account configure <name>` (neu) konfiguriert werden. In diesem Modus überspringt der Assistent die Erkennung: Er verwendet die vorhandenen Werte als Standardwerte für die Eingaben.

Sie können die Konfiguration auch manuell schreiben:

- Kopieren Sie die dokumentierte [./config.sample.toml](./config.sample.toml)
- Fügen Sie sie in einen der folgenden Pfade ein:
  - `$XDG_CONFIG_HOME/himalaya/config.toml`
  - `$HOME/.config/himalaya/config.toml`
  - `$HOME/.himalayarc`
- Kommentieren Sie die gewünschten Optionen ein oder aus

…oder übergeben Sie `-c <PATH>` / setzen Sie `HIMALAYA_CONFIG=<PATH>`. Mehrere Pfade können gleichzeitig übergeben werden, getrennt durch `:`; der erste ist die Basis und die restlichen werden darüber tief zusammengeführt.

## Verwendung

### Gemeinsame API

Backend-unabhängige Befehle arbeiten mit dem ersten konfigurierten Backend des Kontos oder dem mit `-b/--backend` ausgewählten:

```
himalaya mailboxes list
himalaya envelopes list -m INBOX --page 2
himalaya envelopes search from alice and after 2026-01-01 order by date desc
himalaya flags add -m INBOX --flag seen 1:3,5
himalaya messages copy --from INBOX --to Archives 42
himalaya attachments download -m INBOX 42
```

Wenn der Alias `inbox` unter `[mailbox.alias]` konfiguriert ist, wird `-m/--mailbox` optional: Gemeinsame Befehle greifen auf diese ID zurück. Mit `[mailbox.alias] inbox = "INBOX"` verkürzen sich die Aufrufe oben zu `envelopes list --page 2`, `flags add --flag seen 1:3,5` usw.

`envelopes list` ist einfache Paginierung, sortiert nach Datum absteigend. Zum Filtern oder Sortieren verwenden Sie `envelopes search` mit einer abschließenden Abfrage für `date`, `after`, `from`, `to`, `subject`, `body`, `flag`-Bedingungen (kombiniert mit `and`, `or`, `not`, gruppiert mit Klammern) und einer Sortierkette `order by date|from|to|subject [asc|desc]`. Datums-Klauseln beziehen sich auf den `Date:`-Header (Sendezeitpunkt) auf jedem Backend.

Die gemeinsame Oberfläche ist eine strikte kleinst-gemeinsame-Nenner-Teilmenge über IMAP, JMAP und Maildir. Operationen, die sich nicht verallgemeinern lassen (Mailbox-Rollen, Attribut-Flags, JMAP-spezifische Abfragen…), befinden sich unter den protokollspezifischen Unterbefehlen.

### Protokollspezifische APIs

Jedes Backend stellt seine vollständige native API in einer eigenen Untergruppe bereit:

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

Die Flag `-b/--backend` wird nur von den gemeinsamen Befehlen verarbeitet; Protokoll-Unterbefehle verwenden immer ihr eigenes Backend.

### Nachrichten verfassen

Die integrierten Befehle `messages compose` / `reply` / `forward` decken einfache Fälle über CLI-Flags ab:

```
himalaya messages compose --from me@example.org --to you@example.org \
    --subject "Hello" --body "Hi!" --send
```

Für reichhaltigere Komposition (Multipart-MIME, MML-Direktiven, Signierung/Verschlüsselung, Editor-gesteuerte Workflows…), richten Sie einen benutzerdefinierten Composer in `[message.composer.*]` ein und rufen ihn mit den `-with`-Varianten auf. Zum Beispiel mit [`mml`](https://github.com/pimalaya/mml):

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

`messages mailto <URI>` parst eine RFC-6068-`mailto:`-URI (Empfängerliste im Pfad, Query-Parameter `to` / `cc` / `bcc` / `subject` / `body`), erstellt ein RFC-5322-Entwurfsgerüst mit vorausgefüllten Headern und leitet es auf stdin an den benannten (oder Standard-)Composer zur Bearbeitung weiter. Die Ausgabe des Composers wird wie bei den anderen `-with`-Varianten über `--save` / `--send` geroutet. Nützlich als Desktop-`mailto:`-Handler.

### Nachrichten lesen

Der integrierte Befehl `messages read` rendert eine Nachricht mit Himalayas Standard-Formatter. Für benutzerdefiniertes Rendering deklarieren Sie einen Reader in `[message.reader.*]` und rufen `read-with` auf:

```toml
[message.reader.mml]
command = "mml read"
default = true
```

```
himalaya messages read-with -m INBOX 42
```

### Sitzungen wiederverwenden

Jeder Aufruf öffnet standardmäßig eine frische TCP+TLS+SASL-Sitzung. Um den Handshake über viele Befehle zu amortisieren, kombinieren Sie Himalaya mit [`sirup`](https://github.com/pimalaya/sirup): `sirup` stellt eine vorauthentifizierte IMAP/SMTP-Sitzung über einen Unix-Socket bereit, und Himalaya kann seinen `imap.server` / `smtp.server` auf diesen Socket zeigen.

## Schnittstellen

Diese Schnittstellen sind auf Himalaya CLI aufgebaut, um die Benutzererfahrung zu verbessern:

- [pimalaya/himalaya-tui](https://github.com/pimalaya/himalaya-tui): offizielle TUI (in aktiver Entwicklung)
- [pimalaya/himalaya-vim](https://github.com/pimalaya/himalaya-vim): Vim-Plugin
- [dantecatalfamo/himalaya-emacs](https://github.com/dantecatalfamo/himalaya-emacs): Emacs-Plugin
- [jns/himalaya](https://www.raycast.com/jns/himalaya): Raycast-Erweiterung
- [openclaw/openclaw](https://github.com/openclaw/openclaw/blob/main/skills/himalaya/SKILL.md): OpenClaw SKILL
- [parisni/dfzf](https://github.com/parisni/dfzf): dfzf-Integration

## FAQ

<details>
  <summary>Wie unterscheidet es sich von aerc, mutt oder alpine?</summary>

  Aerc, mutt und alpine können als Terminal User Interfaces (TUI) kategorisiert werden. Wenn das Programm ausgeführt wird, ist Ihr Terminal in eine Event-Schleife gesperrt und Sie interagieren mit Ihren E-Mails über Tastenkürzel.

  Himalaya ist eine Command-Line Interface (CLI). Es gibt keine Event-Schleife: Sie interagieren mit Ihren E-Mails über Shell-Befehle, zustandslos.

  Eine dedizierte TUI ([himalaya-tui](https://github.com/pimalaya/himalaya-tui)) wird aktiv auf denselben Pimalaya-Bibliotheken entwickelt.
</details>

<details>
  <summary>Wie werden Secrets aufgelöst?</summary>

  Jedes Feld `*.passwd` / `*.password` / `*.token` akzeptiert entweder ein rohes Literal oder einen Shell-Befehl, der das Secret auf stdout ausgibt. Die rohe Form ist praktisch zum Testen, sollte aber nicht in der Produktion verwendet werden:

  ```toml
  imap.sasl.plain.passwd.raw = "***"
  imap.sasl.plain.passwd.command = "pass show example"
  imap.sasl.plain.passwd.command = ["pass", "show", "example"]
  ```

  Native Keyring-Unterstützung wurde in v2 entfernt. Verwenden Sie [pimalaya/mimosa](https://github.com/pimalaya/mimosa) (oder `pass`, `secret-tool`, `gopass`…) als `command`.
</details>

<details>
  <summary>Wie wird OAuth 2.0 behandelt?</summary>

  v2 enthält keine OAuth-Flows. Verwenden Sie [pimalaya/ortie](https://github.com/pimalaya/ortie) (oder einen anderen Token-Broker), um ein Access Token zu erhalten, und binden Sie es als `command` ein, der das Token auf stdout zurückgibt. Für JMAP zeigen Sie `jmap.auth.bearer.token.command` auf den Broker; für IMAP/SMTP leiten Sie den Bearer über einen SASL-Mechanismus, der ein command-basiertes Passwort verarbeitet.
</details>

<details>
  <summary>Wie erkennt der Assistent IMAP/SMTP/JMAP-Konfigurationen?</summary>

  Der Assistent führt drei Erkennungsmechanismen nacheinander auf der E-Mail-Adressdomäne aus; der erste nicht-leere Treffer gewinnt:

  1. **PACC** <sup>[draft-ietf-mailmaint-pacc-02](https://datatracker.ietf.org/doc/html/draft-ietf-mailmaint-pacc-02)</sup>: Well-known JSON, digest-verifiziert gegen den `_ua-auto-config`-TXT-Eintrag.
  2. **Thunderbird Autoconfiguration**: ISP main / well-known / ISPDB-Lookups, dann MX-basierter Retry, dann die `mailconf=<URL>`-TXT-Weiterleitung.
  3. **RFC 6186 SRV**: `_imap._tcp`, `_imaps._tcp`, `_submission._tcp`-Lookups, zu einem einzelnen Bericht zusammengefasst.

  Siehe [io-discovery](https://github.com/pimalaya/io-discovery) für die vollständige Kette.
</details>

<details>
  <summary>Wie debuggt man Himalaya CLI?</summary>

  Verwenden Sie `--log-level <level>` (Alias `--log`), wobei `<level>` einer von `off`, `error`, `warn`, `info`, `debug`, `trace` ist:

  ```
  himalaya --log trace mailboxes list
  ```

  Die Umgebungsvariable `RUST_LOG` wird berücksichtigt, wenn `--log` nicht übergeben wird, und unterstützt Filter pro Ziel (siehe die [`env_logger`-Dokumentation](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)). `RUST_BACKTRACE=1` aktiviert vollständige Fehler-Backtraces.

  Logs werden nach `stderr` geschrieben und können leicht in eine Datei umgeleitet werden:

  ```
  himalaya --log trace mailboxes list 2>/tmp/himalaya.log
  ```

  Sie können Logs auch direkt in eine Datei senden über `--log-file <path>`:

  ```
  himalaya --log trace --log-file /tmp/himalaya.log mailboxes list
  ```
</details>

<details>
  <summary>Wie deaktiviert man Farbausgabe?</summary>

  Setzen Sie `NO_COLOR=1` in Ihrer Umgebung.
</details>

## Social Media

- Chat auf [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- Neuigkeiten auf [Mastodon](https://fosstodon.org/@pimalaya) oder [RSS](https://fosstodon.org/@pimalaya.rss)
- E-Mail an [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Besonderer Dank an die [NLnet-Stiftung](https://nlnet.nl/) und die [Europäische Kommission](https://www.ngi.eu/), die das Projekt seit Jahren finanziell unterstützen:

- 2022 → 2023: [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024: [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026: [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- *2027 in Vorbereitung…*

Wenn Ihnen das Projekt gefällt, können Sie gerne über einen der folgenden Anbieter spenden:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
