# himalaya architecture

Read the [Pimalaya ARCHITECTURE](https://github.com/pimalaya/.github/blob/master/ARCHITECTURE.md) first: it describes the conventions every Pimalaya repository shares (layering, the sans-I/O coroutine approach, command and config conventions, code style, licensing). This document only covers what is specific to himalaya, and assumes you know that shared context.

If a statement here conflicts with the code, the code wins; please flag it.

## Where himalaya fits

himalaya is an **application**, the top layer of the Pimalaya stack: a CLI to manage emails. It has no library target (only `main.rs`) and writes no protocol or storage logic of its own. It is a thin shell that drives the sans-I/O libraries below it:

- [io-email](https://github.com/pimalaya/io-email): the cross-protocol email domain API (the shared commands);
- [io-imap](https://github.com/pimalaya/io-imap), [io-jmap](https://github.com/pimalaya/io-jmap), [io-gmail](https://github.com/pimalaya/io-gmail), [io-smtp](https://github.com/pimalaya/io-smtp), [io-maildir](https://github.com/pimalaya/io-maildir), [io-m2dir](https://github.com/pimalaya/io-m2dir): the protocol/storage backends;
- [pimconf](https://github.com/pimalaya/pimconf): account discovery (Thunderbird autoconfig, RFC 6186 SRV, RFC 6764 well-known);
- [pimalaya-cli](https://github.com/pimalaya/cli), [pimalaya-config](https://github.com/pimalaya/config), [pimalaya-stream](https://github.com/pimalaya/stream): shared CLI plumbing (clap args, printer, logger), TOML config loading, and the blocking I/O runtime.

All real I/O lives in those libraries; himalaya consumes their blocking `*Std` clients and only orchestrates them and renders results.

## Three command families

The command tree (`cli.rs`, `Command`) is split into three groups:

1. **Shared API** (`mailbox`, `envelope`, `flag`, `message`, `attachment`): the cross-protocol, least-common-denominator surface, served by io-email's `EmailClientStd`. Every operation works the same regardless of which backend serves the active account.
2. **Protocol-specific APIs** (`imap`, `jmap`, `gmail`, `maildir`, `m2dir`, `smtp`): each exposes the full surface of one backend, including operations the shared API cannot model. Each is gated behind its own cargo feature.
3. **Meta** (`account`, `completion`, `manual`): account configuration/inspection, shell completions, man pages.

This is the standard Pimalaya CLI split: a portable shared API plus per-protocol escape hatches.

## Backend selection (shared commands)

The shared commands target a backend chosen by the global `--backend` flag, a `Backend` enum (`backend.rs`): `auto` (default), `imap`, `jmap`, `gmail`, `maildir`, `m2dir`, `smtp`. `auto` lets each shared command pick the first configured-and-allowed backend in io-email's priority order; a named value pins the command to that backend (and bails if the account has no matching config block). `shared/client.rs` builds the io-email `EmailClient` (a wrapper over `EmailClientStd`): it registers every configured-and-allowed backend slot, then io-email's dispatcher routes each call. The wrapper `Deref`s onto `EmailClientStd`, so shared command code calls the io-email API directly, with the merged `Account` threaded alongside as a sibling argument.

## Protocol-specific commands

Each protocol module (`imap/`, `jmap/`, `gmail/`, ...) builds its own backend client via a `build_<proto>_client` helper and a `<Proto>Client` wrapper that `Deref`s onto the underlying io-* `*Std` client, ignoring `--backend`. Subcommands are clap-derived structs carrying their own arguments, with an `execute(self, printer, account, client)` method (the shared nested-execute convention); the module's command enum dispatches to them.

### The `gmail` command

The `gmail` command is organized one-to-one by Gmail REST API resource domain, so it tracks io-gmail directly rather than going through io-email's least-common-denominator shape (io-email's shared commands already cover the LCD over Gmail; this command is the Gmail-native escape hatch). `gmail/client.rs` provides `GmailClient` (wrapping io-gmail's `GmailClientStd`) and `build_gmail_client`; one file per domain holds that domain's subcommands:

- `profile` (users.getProfile), `labels` (users.labels), `messages` (users.messages, including `import`/`insert`/`batch-modify`/`batch-delete`), `attachments` (messages.attachments.get), `drafts` (users.drafts), `threads` (users.threads), `history` (users.history), `settings` (users.settings: vacation, IMAP, POP, language, auto-forwarding, filters, forwarding addresses, delegates, send-as).

Commands drive io-gmail coroutines through `client.run(...)` (and the client's convenience methods for the first-class verbs). `gmail/client.rs` also exports `gmail_token`, shared with `shared/client.rs` and `account/check.rs` so the shared Gmail backend and the protocol command resolve credentials identically. Not yet implemented: `users.watch`/`stop`.

## Command conventions and output

`Command::execute` in `cli.rs` is the single dispatch point: it loads the config (running the wizard if none exists via `load_or_wizard`), selects the account, builds the appropriate client (shared `EmailClient` or a per-protocol client), and hands it to the subcommand.

Output follows the Pimalaya stdout/stderr rule: all data and errors go to stdout through `pimalaya_cli::printer` (with `--output json` switching every command to JSON), stderr carries logs only. A command returns a `Serialize + Display` type to the printer (e.g. a table) or a `Message`, rather than printing inline. Each command's doc comment is its `--help` text, so `himalaya <command> --help` is the canonical usage reference for both humans and AI agents; the README documents no per-command usage.

## Configuration and the wizard

Config is loaded by pimalaya-config from the first existing canonical path (or the `-c` / `HIMALAYA_CONFIG` override), with later paths deep-merged on top. The schema (`config.rs`) is multi-account: a top-level block plus named `[accounts.<name>]` blocks, each carrying optional per-backend sub-blocks (`[imap]`, `[jmap]`, `[gmail]`, `[maildir]`, `[m2dir]`, `[smtp]`). `Account::from(config).merge(Account::from(account_config))` flattens global defaults under the selected account into the runtime `Account` (rendering options, mailbox aliases, downloads dir) every command consumes.

The `[gmail]` block carries `user-id` (default `me`), TLS settings, `alpn` (default `["http/1.1"]`) and an `auth.token` holding an OAuth 2.0 bearer access token, the only authorization Gmail accepts (supplied raw or via a `token.command`). Gmail needs no server address (the API host is fixed) and no token refresh logic (the token is supplied externally). When no config file exists, `load_or_wizard` runs the interactive wizard (`wizard/`) to bootstrap one via discovery (PACC, autoconfig, SRV); the wizard sets up IMAP+SMTP or JMAP accounts, while Gmail accounts are configured by hand.

## Module layout

```
src/
  main.rs            entry point: parse Cli, build printer, dispatch
  cli.rs             Cli/Command, global flags, execute dispatch, load_or_wizard
  backend.rs         Backend enum (auto/imap/jmap/gmail/maildir/m2dir/smtp) + allow rules
  config.rs          TOML schema: Config, AccountConfig, per-backend blocks
  shared/            cross-protocol least-common-denominator commands
    client.rs        EmailClient wrapper (registers backends, dispatches)
    mailbox/ envelope/ flag/ message/ attachment/
  imap/  jmap/  gmail/  maildir/  m2dir/  smtp/   protocol-specific commands
    <proto>/client.rs   build_<proto>_client + <Proto>Client wrapper
  account/           list / check / configure + Account runtime context
  wizard/            first-run interactive config bootstrap (discover, pacc, srv, edit)
```

`shared/` is the portable surface; the per-protocol modules are the escape hatches; `account/` and `wizard/` are the meta and bootstrap concerns.
