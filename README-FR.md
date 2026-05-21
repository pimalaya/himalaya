<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI pour gérer les e-mails</p>
  <p>
    <a href="https://github.com/pimalaya/himalaya/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/pimalaya/himalaya?color=success"/></a>
    <a href="https://repology.org/project/himalaya/versions"><img alt="Repology" src="https://img.shields.io/repology/repositories/himalaya?color=success"></a>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white"/></a>
    <a href="https://fosstodon.org/@pimalaya"><img alt="Mastodon" src="https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white"/></a>
  </p>
  <p>
    <strong>Langues / Languages:</strong>
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
> Ce README documente Himalaya v2, qui **n’est pas encore publié**. Si vous utilisez v1 (`himalaya v1.2.0` ou antérieur), consultez le [README v1.2.0](https://github.com/pimalaya/himalaya/blob/v1.2.0/README.md). Le guide [MIGRATION.md](./MIGRATION.md) accompagne les utilisateurs v1 à travers les changements incompatibles.

## Table des matières

- [Fonctionnalités](#features)
- [Installation](#installation)
  - [Binaire précompilé](#pre-built-binary)
  - [Cargo](#cargo)
  - [Arch Linux](#arch-linux)
  - [Homebrew](#homebrew)
  - [Scoop](#scoop)
  - [Fedora Linux/CentOS/RHEL](#fedora-linuxcentosrhel)
  - [Nix](#nix)
  - [Sources](#sources)
- [Configuration](#configuration)
- [Utilisation](#usage)
  - [API partagée](#shared-api)
  - [APIs spécifiques aux protocoles](#protocol-specific-apis)
  - [Rédiger des messages](#composing-messages)
  - [Lire des messages](#reading-messages)
  - [Réutiliser les sessions](#re-using-sessions)
- [Interfaces](#interfaces)
- [FAQ](#faq)
- [Réseaux sociaux](#social)
- [Parrainage](#sponsoring)

## Fonctionnalités

- **API partagée** qui mappe `mailboxes`, `envelopes`, `flags`, `messages` et `attachments` vers le backend actif
- **APIs spécifiques aux protocoles** exposant la surface complète de chaque backend (`himalaya imap/smtp/maildir/jmap…`)
- Prise en charge **IMAP** <sup>[rfc9051](https://www.iana.org/go/rfc9051)</sup> (nécessite la feature `imap`)
- Prise en charge **JMAP** <sup>[rfc8620](https://www.iana.org/go/rfc8620), [rfc8621](https://www.iana.org/go/rfc8621)</sup> (nécessite la feature `jmap`)
- Prise en charge **Maildir** (nécessite la feature `maildir`)
- Backend **SMTP** <sup>[rfc5321](https://www.iana.org/go/rfc5321)</sup> (nécessite la feature `smtp`)
- Prise en charge **TLS** :
  - [native-tls](https://crates.io/crates/native-tls) (nécessite la feature `native-tls`)
  - [rustls](https://crates.io/crates/rustls) :
    - Fournisseur crypto AWS-LC (nécessite la feature `rustls-aws`)
    - Fournisseur crypto Ring (nécessite la feature `rustls-ring`)
- Prise en charge **SASL** : anonymous, login, plain, oauthbearer, xoauth2, scram-sha-256
- Assistant de **découverte de fournisseur** alimenté par [io-discovery](https://github.com/pimalaya/io-discovery) : Thunderbird Autoconfiguration, PACC et recherches SRV RFC 6186
- Configuration **TOML** avec prise en charge multi-comptes
- Sortie **JSON** via `--json`

*Himalaya CLI est écrit en [Rust](https://www.rust-lang.org/) et s’appuie sur les [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) pour activer ou désactiver des fonctionnalités. Les features par défaut se trouvent dans la section `features` du [`Cargo.toml`](./Cargo.toml#L18), ou sur [docs.rs](https://docs.rs/crate/himalaya/latest/features).*

## Installation

### Binaire précompilé

Himalaya CLI peut être installé avec l’installateur `install.sh` :

*En tant que root :*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh
```

*En tant qu’utilisateur normal :*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
```

Ces commandes installent le dernier binaire depuis la section [releases](https://github.com/pimalaya/himalaya/releases) de GitHub.

Si vous voulez une version plus récente que la dernière release, consultez le workflow GitHub [releases](https://github.com/pimalaya/himalaya/actions/workflows/releases.yml) et cherchez la section *Artifacts*. Vous y trouverez un binaire précompilé pour votre OS. Ces binaires sont construits depuis la branche `master`.

*De tels binaires sont compilés avec les cargo features par défaut. Si vous avez besoin de plus de features, utilisez une autre méthode d’installation.*

### Cargo

Himalaya CLI peut être installé avec [cargo](https://doc.rust-lang.org/cargo/) :

```
cargo install himalaya --locked
```

Avec uniquement la prise en charge IMAP :

```
cargo install himalaya --locked --no-default-features --features imap
```

Vous pouvez aussi utiliser le dépôt git pour une version plus à jour (mais moins stable) :

```
cargo install --locked --git https://github.com/pimalaya/himalaya.git
```

### Arch Linux

Himalaya CLI peut être installé sur [Arch Linux](https://archlinux.org/) via le dépôt communautaire :

```
pacman -S himalaya
```

ou le [dépôt utilisateur](https://aur.archlinux.org/) :

```
git clone https://aur.archlinux.org/himalaya-git.git
cd himalaya-git
makepkg -isc
```

Si vous utilisez [yay](https://github.com/Jguer/yay), c’est encore plus simple :

```
yay -S himalaya-git
```

### Homebrew

Himalaya CLI peut être installé avec [Homebrew](https://brew.sh/) :

```
brew install himalaya
```

Note : les cargo features ne sont pas compatibles avec brew. Si vous avez besoin d’un autre ensemble de features, utilisez une autre méthode d’installation.

### Scoop

Himalaya CLI peut être installé avec [Scoop](https://scoop.sh/) :

```
scoop install himalaya
```

### Fedora Linux/CentOS/RHEL

Himalaya CLI peut être installé sur [Fedora Linux](https://fedoraproject.org/)/CentOS/RHEL via le dépôt [COPR](https://copr.fedorainfracloud.org/coprs/atim/himalaya/) :

```
dnf copr enable atim/himalaya
dnf install himalaya
```

### Nix

Himalaya CLI peut être installé avec [Nix](https://serokell.io/blog/what-is-nix) :

```
nix-env -i himalaya
```

Vous pouvez aussi utiliser le dépôt git pour une version plus à jour (mais moins stable) :

```
nix-env -if https://github.com/pimalaya/himalaya/archive/master.tar.gz
```

*Ou, depuis l’arborescence source clonée :*

```
nix-env -if .
```

Si vous avez la feature [Flakes](https://nixos.wiki/wiki/Flakes) activée :

```
nix profile install github:pimalaya/himalaya
```

*Ou, depuis l’arborescence source clonée :*

```
nix profile install
```

*Vous pouvez aussi exécuter Himalaya sans l’installer :*

```
nix run github:pimalaya/himalaya
```

### Sources

```
git clone https://github.com/pimalaya/himalaya
cd himalaya
nix develop --command cargo build --release
```

*Les binaires sont disponibles sous le dossier `target/release`.*

## Configuration

Lancez simplement `himalaya`. Si aucun fichier de configuration n’est trouvé, l’assistant demande un nom de compte et une adresse e-mail, exécute la [découverte de fournisseur](https://github.com/pimalaya/io-discovery) (PACC → Thunderbird Autoconfiguration → RFC 6186 SRV), remplit les invites IMAP/SMTP (ou JMAP) avec les valeurs découvertes et écrit le résultat sur le disque.

Les comptes peuvent être (re)configurés plus tard avec `himalaya account configure <name>`. Dans ce mode, l’assistant ignore la découverte : il réutilise les valeurs existantes comme valeurs par défaut des invites.

Vous pouvez aussi écrire la configuration à la main :

- Copiez le [./config.sample.toml](./config.sample.toml) documenté
- Collez-le dans l’un des emplacements suivants :
  - `$XDG_CONFIG_HOME/himalaya/config.toml`
  - `$HOME/.config/himalaya/config.toml`
  - `$HOME/.himalayarc`
- Commentez ou décommentez les options souhaitées

…ou passez `-c <PATH>` / définissez `HIMALAYA_CONFIG=<PATH>`. Plusieurs chemins peuvent être passés à la fois, séparés par `:` ; le premier sert de base et les autres sont fusionnés en profondeur par-dessus.

## Utilisation

### API partagée

Les commandes indépendantes du backend opèrent sur le premier backend configuré du compte, ou celui sélectionné avec `-b/--backend` :

```
himalaya mailboxes list
himalaya envelopes list -m INBOX --page 2
himalaya envelopes search from alice and after 2026-01-01 order by date desc
himalaya flags add -m INBOX --flag seen 1:3,5
himalaya messages copy --from INBOX --to Archives 42
himalaya attachments download -m INBOX 42
```

Lorsque l’alias `inbox` est configuré sous `[mailbox.alias]`, `-m/--mailbox` devient optionnel : les commandes partagées retombent sur cet id. Avec `[mailbox.alias] inbox = "INBOX"`, les appels ci-dessus se raccourcissent en `envelopes list --page 2`, `flags add --flag seen 1:3,5`, etc.

`envelopes list` est une pagination simple, triée par date décroissante. Pour filtrer ou trier, utilisez `envelopes search` avec une requête finale couvrant les conditions `date`, `after`, `from`, `to`, `subject`, `body`, `flag` (combinées avec `and`, `or`, `not`, regroupées avec des parenthèses) et une chaîne de tri `order by date|from|to|subject [asc|desc]`. Les clauses de date ciblent l’en-tête `Date:` (date d’envoi) sur tous les backends.

La surface partagée est un sous-ensemble strict du plus petit dénominateur commun entre IMAP, JMAP et Maildir. Les opérations qui ne se généralisent pas (rôles de boîtes, drapeaux d’attributs, requêtes spécifiques JMAP…) vivent sous les sous-commandes spécifiques au protocole.

### APIs spécifiques aux protocoles

Chaque backend expose son API native complète sous son propre sous-groupe :

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

L’option `-b/--backend` n’est consommée que par les commandes partagées ; les sous-commandes de protocole utilisent toujours leur propre backend.

### Rédiger des messages

Les commandes intégrées `messages compose` / `reply` / `forward` couvrent les cas simples via des flags CLI :

```
himalaya messages compose --from me@example.org --to you@example.org \
    --subject "Hello" --body "Hi!" --send
```

Pour une composition plus riche (MIME multipart, directives MML, signature/chiffrement, flux pilotés par l’éditeur…), configurez un composeur défini par l’utilisateur dans `[message.composer.*]` et invoquez-le avec les variantes `-with`. Par exemple, avec [`mml`](https://github.com/pimalaya/mml) :

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

`messages mailto <URI>` analyse une URI `mailto:` RFC 6068 (liste de destinataires dans le chemin, paramètres de requête `to` / `cc` / `bcc` / `subject` / `body`), construit un brouillon RFC 5322 avec ces en-têtes préremplis, puis l’envoie sur stdin au composeur nommé (ou par défaut) pour édition. La sortie du composeur est routée via `--save` / `--send` comme les autres variantes `-with`. Utile comme gestionnaire de bureau `mailto:`.

### Lire des messages

La commande intégrée `messages read` affiche un message avec le formateur par défaut de himalaya. Pour un rendu personnalisé, déclarez un lecteur dans `[message.reader.*]` et appelez `read-with` :

```toml
[message.reader.mml]
command = "mml read"
default = true
```

```
himalaya messages read-with -m INBOX 42
```

### Réutiliser les sessions

Chaque invocation ouvre par défaut une nouvelle session TCP+TLS+SASL. Pour amortir la poignée de main sur de nombreuses commandes, associez himalaya à [`sirup`](https://github.com/pimalaya/sirup) : `sirup` expose une session IMAP/SMTP préauthentifiée sur un socket Unix, et himalaya peut pointer son `imap.server` / `smtp.server` vers ce socket.

## Interfaces

Ces interfaces sont construites au-dessus de Himalaya CLI pour améliorer l’expérience utilisateur :

- [pimalaya/himalaya-tui](https://github.com/pimalaya/himalaya-tui) : TUI officielle (en développement actif)
- [pimalaya/himalaya-vim](https://github.com/pimalaya/himalaya-vim) : extension Vim
- [dantecatalfamo/himalaya-emacs](https://github.com/dantecatalfamo/himalaya-emacs) : extension Emacs
- [jns/himalaya](https://www.raycast.com/jns/himalaya) : extension Raycast
- [openclaw/openclaw](https://github.com/openclaw/openclaw/blob/main/skills/himalaya/SKILL.md) : SKILL OpenClaw
- [parisni/dfzf](https://github.com/parisni/dfzf) : intégration dfzf

## FAQ

<details>
  <summary>En quoi diffère-t-il d’aerc, mutt ou alpine ?</summary>

  aerc, mutt et alpine peuvent être classés comme interfaces utilisateur en terminal (TUI). À l’exécution, le terminal est verrouillé dans une boucle d’événements et vous interagissez avec vos e-mails via des raccourcis clavier.

  Himalaya est une interface en ligne de commande (CLI). Il n’y a pas de boucle d’événements : vous interagissez avec vos e-mails via des commandes shell, de manière sans état.

  Une TUI dédiée ([himalaya-tui](https://github.com/pimalaya/himalaya-tui)) est en développement actif sur les mêmes bibliothèques Pimalaya.
</details>

<details>
  <summary>Comment les secrets sont-ils résolus ?</summary>

  Chaque champ `*.passwd` / `*.password` / `*.token` accepte soit un littéral brut, soit une commande shell qui imprime le secret sur stdout. La forme brute est pratique pour les tests mais ne doit pas être utilisée en production :

  ```toml
  imap.sasl.plain.passwd.raw = "***"
  imap.sasl.plain.passwd.command = "pass show example"
  imap.sasl.plain.passwd.command = ["pass", "show", "example"]
  ```

  La prise en charge native du trousseau a été supprimée en v2. Utilisez [pimalaya/mimosa](https://github.com/pimalaya/mimosa) (ou `pass`, `secret-tool`, `gopass`…) comme `command`.
</details>

<details>
  <summary>Comment OAuth 2.0 est-il géré ?</summary>

  v2 n’embarque pas de flux OAuth. Utilisez [pimalaya/ortie](https://github.com/pimalaya/ortie) (ou tout autre courtier de jetons) pour obtenir un jeton d’accès, puis branchez-le comme `command` renvoyant le jeton sur stdout. Pour JMAP, pointez `jmap.auth.bearer.token.command` vers le courtier ; pour IMAP/SMTP, routez le bearer via un mécanisme SASL qui consomme un mot de passe issu d’une commande.
</details>

<details>
  <summary>Comment l’assistant découvre-t-il les configs IMAP/SMTP/JMAP ?</summary>

  L’assistant exécute trois mécanismes de découverte en série sur le domaine de l’adresse e-mail ; le premier résultat non vide l’emporte :

  1. **PACC** <sup>[draft-ietf-mailmaint-pacc-02](https://datatracker.ietf.org/doc/html/draft-ietf-mailmaint-pacc-02)</sup> : JSON well-known, vérifié par digest contre l’enregistrement TXT `_ua-auto-config`.
  2. **Thunderbird Autoconfiguration** : recherches ISP main / well-known / ISPDB, puis nouvelle tentative basée sur MX, puis la redirection TXT `mailconf=<URL>`.
  3. **RFC 6186 SRV** : recherches `_imap._tcp`, `_imaps._tcp`, `_submission._tcp` assemblées en un seul rapport.

  Voir [io-discovery](https://github.com/pimalaya/io-discovery) pour la chaîne complète.
</details>

<details>
  <summary>Comment déboguer Himalaya CLI ?</summary>

  Utilisez `--log-level <level>` (alias `--log`) où `<level>` est l’un de `off`, `error`, `warn`, `info`, `debug`, `trace` :

  ```
  himalaya --log trace mailboxes list
  ```

  La variable d’environnement `RUST_LOG` est consultée lorsque `--log` n’est pas passé, et prend en charge des filtres par cible (voir la [documentation `env_logger`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)). `RUST_BACKTRACE=1` active les traces d’erreur complètes.

  Les journaux sont écrits sur `stderr`, ils peuvent donc être redirigés facilement vers un fichier :

  ```
  himalaya --log trace mailboxes list 2>/tmp/himalaya.log
  ```

  Vous pouvez aussi envoyer les journaux directement dans un fichier via `--log-file <path>` :

  ```
  himalaya --log trace --log-file /tmp/himalaya.log mailboxes list
  ```
</details>

<details>
  <summary>Comment désactiver la sortie en couleur ?</summary>

  Définissez `NO_COLOR=1` dans votre environnement.
</details>

## Réseaux sociaux

- Discussion sur [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- Actualités sur [Mastodon](https://fosstodon.org/@pimalaya) ou [RSS](https://fosstodon.org/@pimalaya.rss)
- Courriel à [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Parrainage

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Remerciements particuliers à la [fondation NLnet](https://nlnet.nl/) et à la [Commission européenne](https://www.ngi.eu/) qui financent le projet depuis des années :

- 2022 → 2023 : [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024 : [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026 : [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- *2027 en préparation…*

Si vous appréciez le projet, n’hésitez pas à faire un don via l’un des fournisseurs suivants :

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
