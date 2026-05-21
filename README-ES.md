<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI para gestionar correos</p>
  <p>
    <a href="https://github.com/pimalaya/himalaya/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/pimalaya/himalaya?color=success"/></a>
    <a href="https://repology.org/project/himalaya/versions"><img alt="Repology" src="https://img.shields.io/repology/repositories/himalaya?color=success"></a>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white"/></a>
    <a href="https://fosstodon.org/@pimalaya"><img alt="Mastodon" src="https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white"/></a>
  </p>
  <p>
    <strong>Idiomas / Languages:</strong>
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
> Este README documenta Himalaya v2, que **aún no se ha publicado**. Si usas v1 (`himalaya v1.2.0` o anterior), consulta el [README de v1.2.0](https://github.com/pimalaya/himalaya/blob/v1.2.0/README.md). La guía [MIGRATION.md](./MIGRATION.md) explica a los usuarios de v1 los cambios incompatibles.

## Tabla de contenidos

- [Características](#features)
- [Instalación](#installation)
  - [Binario precompilado](#pre-built-binary)
  - [Cargo](#cargo)
  - [Arch Linux](#arch-linux)
  - [Homebrew](#homebrew)
  - [Scoop](#scoop)
  - [Fedora Linux/CentOS/RHEL](#fedora-linuxcentosrhel)
  - [Nix](#nix)
  - [Fuentes](#sources)
- [Configuración](#configuration)
- [Uso](#usage)
  - [API compartida](#shared-api)
  - [APIs específicas por protocolo](#protocol-specific-apis)
  - [Redactar mensajes](#composing-messages)
  - [Leer mensajes](#reading-messages)
  - [Reutilizar sesiones](#re-using-sessions)
- [Interfaces](#interfaces)
- [Preguntas frecuentes](#faq)
- [Redes sociales](#social)
- [Patrocinio](#sponsoring)

## Características

- **API compartida** que mapea `mailboxes`, `envelopes`, `flags`, `messages` y `attachments` al backend activo
- **APIs específicas por protocolo** que exponen la superficie completa de cada backend (`himalaya imap/smtp/maildir/jmap…`)
- Soporte **IMAP** <sup>[rfc9051](https://www.iana.org/go/rfc9051)</sup> (requiere la feature `imap`)
- Soporte **JMAP** <sup>[rfc8620](https://www.iana.org/go/rfc8620), [rfc8621](https://www.iana.org/go/rfc8621)</sup> (requiere la feature `jmap`)
- Soporte **Maildir** (requiere la feature `maildir`)
- Backend **SMTP** <sup>[rfc5321](https://www.iana.org/go/rfc5321)</sup> (requiere la feature `smtp`)
- Soporte **TLS**:
  - [native-tls](https://crates.io/crates/native-tls) (requiere la feature `native-tls`)
  - [rustls](https://crates.io/crates/rustls):
    - Proveedor criptográfico AWS-LC (requiere la feature `rustls-aws`)
    - Proveedor criptográfico Ring (requiere la feature `rustls-ring`)
- Soporte **SASL**: anonymous, login, plain, oauthbearer, xoauth2, scram-sha-256
- Asistente de **descubrimiento de proveedor** impulsado por [io-discovery](https://github.com/pimalaya/io-discovery): Thunderbird Autoconfiguration, PACC y consultas SRV RFC 6186
- Configuración **TOML** con soporte multi-cuenta
- Salida **JSON** mediante `--json`

*Himalaya CLI está escrito en [Rust](https://www.rust-lang.org/) y usa [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) para activar o desactivar funcionalidades. Las features por defecto están en la sección `features` del [`Cargo.toml`](./Cargo.toml#L18) o en [docs.rs](https://docs.rs/crate/himalaya/latest/features).*

## Instalación

### Binario precompilado

Himalaya CLI puede instalarse con el instalador `install.sh`:

*Como root:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh
```

*Como usuario normal:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
```

Estos comandos instalan el último binario de la sección [releases](https://github.com/pimalaya/himalaya/releases) de GitHub.

Si necesitas una versión más reciente que el último release, revisa el flujo de trabajo [releases](https://github.com/pimalaya/himalaya/actions/workflows/releases.yml) en GitHub y busca la sección *Artifacts*. Encontrarás un binario precompilado para tu SO. Esos binarios se construyen desde la rama `master`.

*Dichos binarios se compilan con las cargo features por defecto. Si necesitas más features, usa otro método de instalación.*

### Cargo

Himalaya CLI puede instalarse con [cargo](https://doc.rust-lang.org/cargo/):

```
cargo install himalaya --locked
```

Solo con soporte IMAP:

```
cargo install himalaya --locked --no-default-features --features imap
```

También puedes usar el repositorio git para una versión más actualizada (pero menos estable):

```
cargo install --locked --git https://github.com/pimalaya/himalaya.git
```

### Arch Linux

Himalaya CLI puede instalarse en [Arch Linux](https://archlinux.org/) con el repositorio comunitario:

```
pacman -S himalaya
```

o con el [repositorio de usuarios](https://aur.archlinux.org/):

```
git clone https://aur.archlinux.org/himalaya-git.git
cd himalaya-git
makepkg -isc
```

Si usas [yay](https://github.com/Jguer/yay), es aún más sencillo:

```
yay -S himalaya-git
```

### Homebrew

Himalaya CLI puede instalarse con [Homebrew](https://brew.sh/):

```
brew install himalaya
```

Nota: las cargo features no son compatibles con brew. Si necesitas otro conjunto de features, usa otro método de instalación.

### Scoop

Himalaya CLI puede instalarse con [Scoop](https://scoop.sh/):

```
scoop install himalaya
```

### Fedora Linux/CentOS/RHEL

Himalaya CLI puede instalarse en [Fedora Linux](https://fedoraproject.org/)/CentOS/RHEL mediante el repositorio [COPR](https://copr.fedorainfracloud.org/coprs/atim/himalaya/):

```
dnf copr enable atim/himalaya
dnf install himalaya
```

### Nix

Himalaya CLI puede instalarse con [Nix](https://serokell.io/blog/what-is-nix):

```
nix-env -i himalaya
```

También puedes usar el repositorio git para una versión más actualizada (pero menos estable):

```
nix-env -if https://github.com/pimalaya/himalaya/archive/master.tar.gz
```

*O, desde el árbol de fuentes clonado:*

```
nix-env -if .
```

Si tienes la feature [Flakes](https://nixos.wiki/wiki/Flakes) activada:

```
nix profile install github:pimalaya/himalaya
```

*O, desde el árbol de fuentes clonado:*

```
nix profile install
```

*También puedes ejecutar Himalaya sin instalarlo:*

```
nix run github:pimalaya/himalaya
```

### Fuentes

```
git clone https://github.com/pimalaya/himalaya
cd himalaya
nix develop --command cargo build --release
```

*Los binarios están en la carpeta `target/release`.*

## Configuración

Ejecuta `himalaya`. Si no hay archivo de configuración, el asistente pide nombre de cuenta y dirección de correo, ejecuta el [descubrimiento de proveedor](https://github.com/pimalaya/io-discovery) (PACC → Thunderbird Autoconfiguration → RFC 6186 SRV), rellena las preguntas IMAP/SMTP (o JMAP) con los valores descubiertos y escribe el resultado en disco.

Las cuentas pueden (re)configurarse después con `himalaya account configure <name>`. En este modo el asistente omite el descubrimiento: reutiliza los valores existentes como valores por defecto de las preguntas.

También puedes escribir la configuración a mano:

- Copia el [./config.sample.toml](./config.sample.toml) documentado
- Pégalo en uno de:
  - `$XDG_CONFIG_HOME/himalaya/config.toml`
  - `$HOME/.config/himalaya/config.toml`
  - `$HOME/.himalayarc`
- Comenta o descomenta las opciones que quieras

…o pasa `-c <PATH>` / establece `HIMALAYA_CONFIG=<PATH>`. Puedes pasar varias rutas a la vez, separadas por `:`; la primera es la base y el resto se fusionan en profundidad encima.

## Uso

### API compartida

Los comandos independientes del backend operan sobre el primer backend configurado de la cuenta, o el seleccionado con `-b/--backend`:

```
himalaya mailboxes list
himalaya envelopes list -m INBOX --page 2
himalaya envelopes search from alice and after 2026-01-01 order by date desc
himalaya flags add -m INBOX --flag seen 1:3,5
himalaya messages copy --from INBOX --to Archives 42
himalaya attachments download -m INBOX 42
```

Cuando el alias `inbox` está configurado bajo `[mailbox.alias]`, `-m/--mailbox` pasa a ser opcional: los comandos compartidos usan ese id por defecto. Con `[mailbox.alias] inbox = "INBOX"`, las llamadas anteriores se acortan a `envelopes list --page 2`, `flags add --flag seen 1:3,5`, etc.

`envelopes list` es paginación simple, ordenada por fecha descendente. Para filtrar u ordenar, usa `envelopes search` con una consulta final que cubre condiciones `date`, `after`, `from`, `to`, `subject`, `body`, `flag` (combinadas con `and`, `or`, `not`, agrupadas con paréntesis) y una cadena de orden `order by date|from|to|subject [asc|desc]`. Las cláusulas de fecha apuntan a la cabecera `Date:` (fecha de envío) en todos los backends.

La superficie compartida es un subconjunto estricto del mínimo común denominador entre IMAP, JMAP y Maildir. Las operaciones que no se generalizan (roles de buzón, flags de atributos, consultas específicas de JMAP…) viven bajo los subcomandos específicos del protocolo.

### APIs específicas por protocolo

Cada backend expone su API nativa completa bajo su propio subgrupo:

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

La opción `-b/--backend` solo la consumen los comandos compartidos; los subcomandos de protocolo siempre usan su propio backend.

### Redactar mensajes

Los comandos integrados `messages compose` / `reply` / `forward` cubren casos simples mediante flags de la CLI:

```
himalaya messages compose --from me@example.org --to you@example.org \
    --subject "Hello" --body "Hi!" --send
```

Para composición más rica (MIME multiparte, directivas MML, firma/cifrado, flujos con editor…), configura un compositor definido por el usuario en `[message.composer.*]` e invócalo con las variantes `-with`. Por ejemplo, con [`mml`](https://github.com/pimalaya/mml):

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

`messages mailto <URI>` analiza una URI `mailto:` RFC 6068 (lista de destinatarios en la ruta, parámetros de consulta `to` / `cc` / `bcc` / `subject` / `body`), construye un borrador RFC 5322 con esas cabeceras prellenadas y lo envía por stdin al compositor nombrado (o por defecto) para editar. La salida del compositor se enruta con `--save` / `--send` como las otras variantes `-with`. Útil como manejador de escritorio `mailto:`.

### Leer mensajes

El comando integrado `messages read` renderiza un mensaje con el formateador por defecto de himalaya. Para renderizado personalizado, declara un lector en `[message.reader.*]` y llama a `read-with`:

```toml
[message.reader.mml]
command = "mml read"
default = true
```

```
himalaya messages read-with -m INBOX 42
```

### Reutilizar sesiones

Cada invocación abre por defecto una sesión TCP+TLS+SASL nueva. Para amortizar el handshake en muchos comandos, combina himalaya con [`sirup`](https://github.com/pimalaya/sirup): `sirup` expone una sesión IMAP/SMTP preautenticada por un socket Unix, y himalaya puede apuntar su `imap.server` / `smtp.server` a ese socket.

## Interfaces

Estas interfaces se construyen sobre Himalaya CLI para mejorar la experiencia de usuario:

- [pimalaya/himalaya-tui](https://github.com/pimalaya/himalaya-tui): TUI oficial (en desarrollo activo)
- [pimalaya/himalaya-vim](https://github.com/pimalaya/himalaya-vim): complemento Vim
- [dantecatalfamo/himalaya-emacs](https://github.com/dantecatalfamo/himalaya-emacs): complemento Emacs
- [jns/himalaya](https://www.raycast.com/jns/himalaya): extensión Raycast
- [openclaw/openclaw](https://github.com/openclaw/openclaw/blob/main/skills/himalaya/SKILL.md): SKILL OpenClaw
- [parisni/dfzf](https://github.com/parisni/dfzf): integración dfzf

## Preguntas frecuentes

<details>
  <summary>¿En qué se diferencia de aerc, mutt o alpine?</summary>

  aerc, mutt y alpine pueden clasificarse como interfaces de usuario en terminal (TUI). Al ejecutar el programa, la terminal queda bloqueada en un bucle de eventos e interactúas con el correo mediante atajos de teclado.

  Himalaya es una interfaz de línea de comandos (CLI). No hay bucle de eventos: interactúas con el correo mediante comandos de shell, de forma sin estado.

  Una TUI dedicada ([himalaya-tui](https://github.com/pimalaya/himalaya-tui)) está en desarrollo activo sobre las mismas bibliotecas Pimalaya.
</details>

<details>
  <summary>¿Cómo se resuelven los secretos?</summary>

  Cada campo `*.passwd` / `*.password` / `*.token` acepta un literal en bruto o un comando shell que imprime el secreto en stdout. La forma en bruto es cómoda para pruebas pero no debe usarse en producción:

  ```toml
  imap.sasl.plain.passwd.raw = "***"
  imap.sasl.plain.passwd.command = "pass show example"
  imap.sasl.plain.passwd.command = ["pass", "show", "example"]
  ```

  El soporte nativo del llavero se eliminó en v2. Usa [pimalaya/mimosa](https://github.com/pimalaya/mimosa) (o `pass`, `secret-tool`, `gopass`…) como `command`.
</details>

<details>
  <summary>¿Cómo se gestiona OAuth 2.0?</summary>

  v2 no incluye flujos OAuth. Usa [pimalaya/ortie](https://github.com/pimalaya/ortie) (u otro intermediario de tokens) para obtener un token de acceso y conéctalo como `command` que devuelve el token en stdout. Para JMAP, apunta `jmap.auth.bearer.token.command` al intermediario; para IMAP/SMTP, enruta el bearer por un mecanismo SASL que consuma una contraseña obtenida por comando.
</details>

<details>
  <summary>¿Cómo descubre el asistente las configuraciones IMAP/SMTP/JMAP?</summary>

  El asistente ejecuta tres mecanismos de descubrimiento en serie sobre el dominio del correo; gana el primer resultado no vacío:

  1. **PACC** <sup>[draft-ietf-mailmaint-pacc-02](https://datatracker.ietf.org/doc/html/draft-ietf-mailmaint-pacc-02)</sup>: JSON well-known, verificado por digest frente al registro TXT `_ua-auto-config`.
  2. **Thunderbird Autoconfiguration**: consultas ISP main / well-known / ISPDB, reintento basado en MX, luego la redirección TXT `mailconf=<URL>`.
  3. **RFC 6186 SRV**: consultas `_imap._tcp`, `_imaps._tcp`, `_submission._tcp` ensambladas en un único informe.

  Consulta [io-discovery](https://github.com/pimalaya/io-discovery) para la cadena completa.
</details>

<details>
  <summary>¿Cómo depurar Himalaya CLI?</summary>

  Usa `--log-level <level>` (alias `--log`) donde `<level>` es uno de `off`, `error`, `warn`, `info`, `debug`, `trace`:

  ```
  himalaya --log trace mailboxes list
  ```

  La variable de entorno `RUST_LOG` se consulta cuando no se pasa `--log`, y admite filtros por objetivo (véase la [documentación de `env_logger`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)). `RUST_BACKTRACE=1` activa trazas de error completas.

  Los registros se escriben en `stderr`, así que pueden redirigirse fácilmente a un archivo:

  ```
  himalaya --log trace mailboxes list 2>/tmp/himalaya.log
  ```

  También puedes enviar los registros directamente a un archivo con `--log-file <path>`:

  ```
  himalaya --log trace --log-file /tmp/himalaya.log mailboxes list
  ```
</details>

<details>
  <summary>¿Cómo desactivar la salida en color?</summary>

  Establece `NO_COLOR=1` en tu entorno.
</details>

## Redes sociales

- Chat en [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- Noticias en [Mastodon](https://fosstodon.org/@pimalaya) o [RSS](https://fosstodon.org/@pimalaya.rss)
- Correo a [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Patrocinio

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Agradecimiento especial a la [fundación NLnet](https://nlnet.nl/) y la [Comisión Europea](https://www.ngi.eu/), que han financiado el proyecto durante años:

- 2022 → 2023: [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024: [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026: [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- *2027 en preparación…*

Si valoras el proyecto, puedes donar mediante uno de estos proveedores:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
