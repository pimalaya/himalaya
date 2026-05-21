<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya</h1>
  <p>CLI para gerenciar e-mails</p>
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
> Este README documenta o Himalaya v2, que **ainda não foi lançado**. Se você estiver usando a v1 (`himalaya v1.2.0` ou anterior), consulte o [README da v1.2.0](https://github.com/pimalaya/himalaya/blob/v1.2.0/README.md). O guia [MIGRATION.md](./MIGRATION.md) orienta usuários da v1 pelas mudanças incompatíveis.

## Índice

- [Recursos](#recursos)
- [Instalação](#instalação)
  - [Binário pré-compilado](#binário-pré-compilado)
  - [Cargo](#cargo)
  - [Arch Linux](#arch-linux)
  - [Homebrew](#homebrew)
  - [Scoop](#scoop)
  - [Fedora Linux/CentOS/RHEL](#fedora-linuxcentosrhel)
  - [Nix](#nix)
  - [Código-fonte](#código-fonte)
- [Configuração](#configuração)
- [Uso](#uso)
  - [API compartilhada](#api-compartilhada)
  - [APIs específicas por protocolo](#apis-específicas-por-protocolo)
  - [Composição de mensagens](#composição-de-mensagens)
  - [Leitura de mensagens](#leitura-de-mensagens)
  - [Reutilização de sessões](#reutilização-de-sessões)
- [Interfaces](#interfaces)
- [FAQ](#faq)
- [Redes sociais](#redes-sociais)
- [Patrocínio](#patrocínio)

## Recursos

- **API compartilhada** que mapeia `mailboxes`, `envelopes`, `flags`, `messages` e `attachments` para o backend ativo
- **APIs específicas por protocolo** expondo a superfície completa de cada backend (`himalaya imap/smtp/maildir/jmap…`)
- Suporte a **IMAP** <sup>[rfc9051](https://www.iana.org/go/rfc9051)</sup> (requer o recurso `imap`)
- Suporte a **JMAP** <sup>[rfc8620](https://www.iana.org/go/rfc8620), [rfc8621](https://www.iana.org/go/rfc8621)</sup> (requer o recurso `jmap`)
- Suporte a **Maildir** (requer o recurso `maildir`)
- Backend **SMTP** <sup>[rfc5321](https://www.iana.org/go/rfc5321)</sup> (requer o recurso `smtp`)
- Suporte a **TLS**:
  - [native-tls](https://crates.io/crates/native-tls) (requer o recurso `native-tls`)
  - [rustls](https://crates.io/crates/rustls):
    - Provedor criptográfico AWS-LC (requer o recurso `rustls-aws`)
    - Provedor criptográfico Ring (requer o recurso `rustls-ring`)
- Suporte a **SASL**: anonymous, login, plain, oauthbearer, xoauth2, scram-sha-256
- Assistente de **descoberta de provedores** baseado em [io-discovery](https://github.com/pimalaya/io-discovery): Thunderbird Autoconfiguration, PACC e consultas SRV RFC 6186
- Configuração **TOML** com suporte a múltiplas contas
- Saída **JSON** via `--json`

*O Himalaya CLI é escrito em [Rust](https://www.rust-lang.org/) e depende de [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) para habilitar ou desabilitar funcionalidades. Os recursos padrão podem ser encontrados na seção `features` do [`Cargo.toml`](./Cargo.toml#L18) ou em [docs.rs](https://docs.rs/crate/himalaya/latest/features).*

## Instalação

### Binário pré-compilado

O Himalaya CLI pode ser instalado com o instalador `install.sh`:

*Como root:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | sudo sh
```

*Como usuário comum:*

```
curl -sSL https://raw.githubusercontent.com/pimalaya/himalaya/master/install.sh | PREFIX=~/.local sh
```

Esses comandos instalam o binário mais recente da seção [releases](https://github.com/pimalaya/himalaya/releases) do GitHub.

Se você quiser uma versão mais atual do que a última release, consulte o workflow [releases](https://github.com/pimalaya/himalaya/actions/workflows/releases.yml) do GitHub e procure a seção *Artifacts*. Você encontrará um binário pré-compilado compatível com o seu SO. Esses binários são compilados a partir do branch `master`.

*Esses binários são compilados com os cargo features padrão. Se precisar de mais recursos, use outro método de instalação.*

### Cargo

O Himalaya CLI pode ser instalado com [cargo](https://doc.rust-lang.org/cargo/):

```
cargo install himalaya --locked
```

Somente com suporte a IMAP:

```
cargo install himalaya --locked --no-default-features --features imap
```

Você também pode usar o repositório git para uma versão mais atual (porém menos estável):

```
cargo install --locked --git https://github.com/pimalaya/himalaya.git
```

### Arch Linux

O Himalaya CLI pode ser instalado no [Arch Linux](https://archlinux.org/) pelo repositório community:

```
pacman -S himalaya
```

ou pelo [repositório de usuários](https://aur.archlinux.org/):

```
git clone https://aur.archlinux.org/himalaya-git.git
cd himalaya-git
makepkg -isc
```

Se você usa [yay](https://github.com/Jguer/yay), é ainda mais simples:

```
yay -S himalaya-git
```

### Homebrew

O Himalaya CLI pode ser instalado com [Homebrew](https://brew.sh/):

```
brew install himalaya
```

Nota: cargo features não são compatíveis com o brew. Se precisar de um conjunto diferente de recursos, use outro método de instalação.

### Scoop

O Himalaya CLI pode ser instalado com [Scoop](https://scoop.sh/):

```
scoop install himalaya
```

### Fedora Linux/CentOS/RHEL

O Himalaya CLI pode ser instalado no [Fedora Linux](https://fedoraproject.org/)/CentOS/RHEL pelo repositório [COPR](https://copr.fedorainfracloud.org/coprs/atim/himalaya/):

```
dnf copr enable atim/himalaya
dnf install himalaya
```

### Nix

O Himalaya CLI pode ser instalado com [Nix](https://serokell.io/blog/what-is-nix):

```
nix-env -i himalaya
```

Você também pode usar o repositório git para uma versão mais atual (porém menos estável):

```
nix-env -if https://github.com/pimalaya/himalaya/archive/master.tar.gz
```

*Ou, a partir do checkout da árvore de código-fonte:*

```
nix-env -if .
```

Se você tiver o recurso [Flakes](https://nixos.wiki/wiki/Flakes) habilitado:

```
nix profile install github:pimalaya/himalaya
```

*Ou, a partir do checkout da árvore de código-fonte:*

```
nix profile install
```

*Você também pode executar o Himalaya diretamente sem instalá-lo:*

```
nix run github:pimalaya/himalaya
```

### Código-fonte

```
git clone https://github.com/pimalaya/himalaya
cd himalaya
nix develop --command cargo build --release
```

*Os binários ficam disponíveis na pasta `target/release`.*

## Configuração

Basta executar `himalaya`. Quando nenhum arquivo de configuração é encontrado, o assistente solicita um nome de conta e endereço de e-mail, executa a [descoberta de provedores](https://github.com/pimalaya/io-discovery) (PACC → Thunderbird Autoconfiguration → RFC 6186 SRV), preenche os prompts IMAP/SMTP (ou JMAP) com os valores descobertos e grava o resultado em disco.

As contas podem ser (re)configuradas depois com `himalaya account configure <name>`. Nesse modo, o assistente pula a descoberta: reutiliza os valores existentes como padrões dos prompts.

Você também pode escrever a configuração manualmente:

- Copie o [./config.sample.toml](./config.sample.toml) documentado
- Cole em um dos seguintes locais:
  - `$XDG_CONFIG_HOME/himalaya/config.toml`
  - `$HOME/.config/himalaya/config.toml`
  - `$HOME/.himalayarc`
- Comente ou descomente as opções desejadas

…ou passe `-c <PATH>` / defina `HIMALAYA_CONFIG=<PATH>`. Vários caminhos podem ser passados de uma vez, separados por `:`; o primeiro é a base e os demais são mesclados profundamente por cima.

## Uso

### API compartilhada

Comandos independentes de backend operam no primeiro backend configurado da conta, ou no selecionado com `-b/--backend`:

```
himalaya mailboxes list
himalaya envelopes list -m INBOX --page 2
himalaya envelopes search from alice and after 2026-01-01 order by date desc
himalaya flags add -m INBOX --flag seen 1:3,5
himalaya messages copy --from INBOX --to Archives 42
himalaya attachments download -m INBOX 42
```

Quando o alias `inbox` está configurado em `[mailbox.alias]`, `-m/--mailbox` torna-se opcional: os comandos compartilhados recorrem a esse id. Com `[mailbox.alias] inbox = "INBOX"`, as chamadas acima se reduzem a `envelopes list --page 2`, `flags add --flag seen 1:3,5`, etc.

`envelopes list` é paginação simples, ordenada por data decrescente. Para filtrar ou ordenar, use `envelopes search` com uma consulta final cobrindo condições `date`, `after`, `from`, `to`, `subject`, `body`, `flag` (combinadas com `and`, `or`, `not`, agrupadas com parênteses) e uma cadeia de ordenação `order by date|from|to|subject [asc|desc]`. Cláusulas de data visam o cabeçalho `Date:` (data de envio) em todos os backends.

A superfície compartilhada é um subconjunto estrito do menor denominador comum entre IMAP, JMAP e Maildir. Operações que não generalizam (papéis de mailbox, flags de atributo, consultas específicas de JMAP…) ficam nos subcomandos específicos por protocolo.

### APIs específicas por protocolo

Cada backend expõe sua API nativa completa em seu próprio subgrupo:

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

A flag `-b/--backend` é consumida apenas pelos comandos compartilhados; subcomandos de protocolo sempre usam seu próprio backend.

### Composição de mensagens

Os comandos integrados `messages compose` / `reply` / `forward` cobrem casos simples via flags da CLI:

```
himalaya messages compose --from me@example.org --to you@example.org \
    --subject "Hello" --body "Hi!" --send
```

Para composição mais rica (MIME multipart, diretivas MML, assinatura/criptografia, fluxos de trabalho com editor…), configure um compositor definido pelo usuário em `[message.composer.*]` e invoque-o com as variantes `-with`. Por exemplo, com [`mml`](https://github.com/pimalaya/mml):

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

`messages mailto <URI>` analisa uma URI RFC 6068 `mailto:` (lista de destinatários no caminho, parâmetros de consulta `to` / `cc` / `bcc` / `subject` / `body`), constrói um esqueleto RFC 5322 de rascunho com esses cabeçalhos pré-preenchidos e o envia via stdin para o compositor nomeado (ou padrão) para edição. A saída do compositor é encaminhada por `--save` / `--send` como nas outras variantes `-with`. Útil como manipulador de `mailto:` no desktop.

### Leitura de mensagens

O comando integrado `messages read` renderiza uma mensagem com o formatador padrão do himalaya. Para renderização personalizada, declare um leitor em `[message.reader.*]` e chame `read-with`:

```toml
[message.reader.mml]
command = "mml read"
default = true
```

```
himalaya messages read-with -m INBOX 42
```

### Reutilização de sessões

Cada invocação abre uma sessão TCP+TLS+SASL nova por padrão. Para amortizar o handshake em muitos comandos, combine o himalaya com [`sirup`](https://github.com/pimalaya/sirup): `sirup` expõe uma sessão IMAP/SMTP pré-autenticada por um socket Unix, e o himalaya pode apontar seu `imap.server` / `smtp.server` para esse socket.

## Interfaces

Essas interfaces são construídas sobre o Himalaya CLI para melhorar a experiência do usuário:

- [pimalaya/himalaya-tui](https://github.com/pimalaya/himalaya-tui): TUI oficial (em desenvolvimento ativo)
- [pimalaya/himalaya-vim](https://github.com/pimalaya/himalaya-vim): plugin Vim
- [dantecatalfamo/himalaya-emacs](https://github.com/dantecatalfamo/himalaya-emacs): plugin Emacs
- [jns/himalaya](https://www.raycast.com/jns/himalaya): extensão Raycast
- [openclaw/openclaw](https://github.com/openclaw/openclaw/blob/main/skills/himalaya/SKILL.md): SKILL OpenClaw
- [parisni/dfzf](https://github.com/parisni/dfzf): integração dfzf

## FAQ

<details>
  <summary>Qual a diferença em relação ao aerc, mutt ou alpine?</summary>

  Aerc, mutt e alpine podem ser categorizados como Interfaces de Usuário de Terminal (TUI). Quando o programa é executado, seu terminal fica bloqueado em um loop de eventos e você interage com seus e-mails usando atalhos de teclado.

  O Himalaya é uma Interface de Linha de Comando (CLI). Não há loop de eventos: você interage com seus e-mails usando comandos de shell, de forma stateless.

  Uma TUI dedicada ([himalaya-tui](https://github.com/pimalaya/himalaya-tui)) está em desenvolvimento ativo sobre as mesmas bibliotecas Pimalaya.
</details>

<details>
  <summary>Como os segredos são resolvidos?</summary>

  Todo campo `*.passwd` / `*.password` / `*.token` aceita um literal bruto ou um comando shell que imprime o segredo em stdout. A forma bruta é conveniente para testes, mas não deve ser usada em produção:

  ```toml
  imap.sasl.plain.passwd.raw = "***"
  imap.sasl.plain.passwd.command = "pass show example"
  imap.sasl.plain.passwd.command = ["pass", "show", "example"]
  ```

  O suporte nativo a keyring foi removido na v2. Use [pimalaya/mimosa](https://github.com/pimalaya/mimosa) (ou `pass`, `secret-tool`, `gopass`…) como `command`.
</details>

<details>
  <summary>Como o OAuth 2.0 é tratado?</summary>

  A v2 não inclui fluxos OAuth. Use [pimalaya/ortie](https://github.com/pimalaya/ortie) (ou qualquer outro broker de tokens) para obter um access token e conecte-o como um `command` que retorna o token em stdout. Para JMAP, aponte `jmap.auth.bearer.token.command` para o broker; para IMAP/SMTP, encaminhe o bearer por um mecanismo SASL que consome uma senha obtida via command.
</details>

<details>
  <summary>Como o assistente descobre configurações IMAP/SMTP/JMAP?</summary>

  O assistente executa três mecanismos de descoberta em série no domínio do endereço de e-mail; o primeiro resultado não vazio prevalece:

  1. **PACC** <sup>[draft-ietf-mailmaint-pacc-02](https://datatracker.ietf.org/doc/html/draft-ietf-mailmaint-pacc-02)</sup>: JSON well-known, verificado por digest contra o registro TXT `_ua-auto-config`.
  2. **Thunderbird Autoconfiguration**: consultas ISP main / well-known / ISPDB, depois nova tentativa baseada em MX, depois o redirecionamento TXT `mailconf=<URL>`.
  3. **RFC 6186 SRV**: consultas `_imap._tcp`, `_imaps._tcp`, `_submission._tcp` reunidas em um único relatório.

  Consulte [io-discovery](https://github.com/pimalaya/io-discovery) para a cadeia completa.
</details>

<details>
  <summary>Como depurar o Himalaya CLI?</summary>

  Use `--log-level <level>` (alias `--log`) onde `<level>` é um de `off`, `error`, `warn`, `info`, `debug`, `trace`:

  ```
  himalaya --log trace mailboxes list
  ```

  A variável de ambiente `RUST_LOG` é consultada quando `--log` não é passado e suporta filtros por alvo (consulte a [documentação do `env_logger`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)). `RUST_BACKTRACE=1` habilita backtraces completos de erro.

  Os logs são escritos em `stderr`, então podem ser redirecionados facilmente para um arquivo:

  ```
  himalaya --log trace mailboxes list 2>/tmp/himalaya.log
  ```

  Você também pode enviar logs diretamente para um arquivo via `--log-file <path>`:

  ```
  himalaya --log trace --log-file /tmp/himalaya.log mailboxes list
  ```
</details>

<details>
  <summary>Como desabilitar a saída colorida?</summary>

  Defina `NO_COLOR=1` no seu ambiente.
</details>

## Redes sociais

- Chat no [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- Notícias no [Mastodon](https://fosstodon.org/@pimalaya) ou [RSS](https://fosstodon.org/@pimalaya.rss)
- E-mail em [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Patrocínio

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Agradecimentos especiais à [fundação NLnet](https://nlnet.nl/) e à [Comissão Europeia](https://www.ngi.eu/) que vêm apoiando financeiramente o projeto há anos:

- 2022 → 2023: [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024: [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026: [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- *2027 em preparação…*

Se você aprecia o projeto, sinta-se à vontade para doar usando um dos seguintes provedores:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
