//! # himalaya
//!
//! CLI to manage emails. himalaya is an application, the top layer of
//! the Pimalaya stack: it writes no protocol or storage logic of its
//! own and ships no library target, only this binary. It is a thin
//! shell driving the sans-I/O io-* libraries below it, consuming their
//! blocking `*Std` clients and orchestrating and rendering the results.
//!
//! ## Backends and plumbing
//!
//! The network backends are io-imap, io-jmap, io-gmail, io-msgraph and
//! io-smtp; ManageSieve is implemented in the optional `sieve` module;
//! the local storage backends are io-maildir and io-m2dir.
//! Account discovery comes from io-pim-discovery (Mozilla autoconfig,
//! PACC, RFC 6186 SRV, RFC 8620 JMAP resolve). The CLI plumbing (clap
//! args, printer, logger), TOML config loading and the blocking stream
//! runtime come from pimalaya-cli, pimalaya-config and pimalaya-stream.
//! Every backend sits behind its own cargo feature, so a build ships
//! only the protocols it needs.
//!
//! ## Command families
//!
//! The command tree ([`cli`], `Command`) splits into three groups. The
//! shared API (mailbox, envelope, flag, message, attachment) is the
//! cross-protocol least-common-denominator surface, behaving the same
//! whatever backend serves the active account. The protocol-specific
//! APIs (imap, jmap, gmail, msgraph, maildir, m2dir, smtp, sieve) each expose
//! the full surface of one backend, including operations the shared API
//! cannot model. The meta commands (account, completion, manual,
//! json-schema) cover account configuration, shell completions, man
//! pages and JSON Schemas.
//!
//! ## Shared commands and backend selection
//!
//! The shared commands run over a local [`shared::client`] `EmailClient`
//! that owns one `BackendClient` enum variant per compiled-in backend:
//! the first configured storage backend the global `--backend` flag
//! allows (local before network), plus an optional SMTP transport for
//! storage backends that cannot send (IMAP, Maildir, m2dir). Each shared
//! method matches the active backend and calls its per-protocol
//! `backend.rs` adapter, which converts io-* results into the CLI's own
//! [`email`] shared types. The active [`account`] context is threaded as
//! a sibling argument through every `execute` chain.
//!
//! ## Protocol-specific commands
//!
//! Each protocol module builds its client via a `build_<proto>_client`
//! helper and a `<Proto>Client` wrapper that derefs onto the io-* `*Std`
//! client, ignoring `--backend`. Subcommands are clap-derived structs
//! with an `execute` method the module's command enum dispatches to. The
//! imap command mirrors IMAP's flat command list; gmail and msgraph
//! track their REST resource domains one-to-one; the filesystem backends
//! expose only operations that map to their on-disk layout, leaving MIME
//! rendering to the shared commands.
//!
//! ## Configuration and output
//!
//! Config is loaded by pimalaya-config from the first existing canonical
//! path (or the `-c` override), later paths deep-merged on top; the
//! schema ([`config`]) is multi-account, a top-level block plus named
//! account blocks carrying optional per-backend sub-blocks. Bare
//! `himalaya` (no subcommand) runs the interactive [`wizard`], which
//! discovers an account and offers to save it to a config file (or
//! prints it on stdout when redirected); it is also proposed when a
//! command finds no config. Bare `himalaya --account <NAME>` shows the
//! help instead. A config that exists but lacks the requested account
//! is a hard error. Output follows the Pimalaya rule: data and errors go
//! to stdout through the printer (`--json` switches every command to JSON),
//! stderr carries logs only. Each command's doc comment is its `--help`
//! text, so `himalaya <command> --help` is the canonical per-command
//! usage reference. The design memory lives in the cairn/ folder (the
//! Cairn convention: spec/, changes/, log/), including the manual
//! provider test reports under cairn/spec/testing/.

mod account;
mod backend;
mod cli;
mod config;
mod email;
#[cfg(feature = "gmail")]
mod gmail;
#[cfg(feature = "imap")]
mod imap;
#[cfg(feature = "jmap")]
mod jmap;
mod json_schema;
#[cfg(feature = "m2dir")]
mod m2dir;
#[cfg(feature = "maildir")]
mod maildir;
#[cfg(feature = "msgraph")]
mod msgraph;
#[cfg(feature = "pimdir")]
mod pimdir;
mod shared;
#[cfg(feature = "sieve")]
mod sieve;
#[cfg(feature = "smtp")]
mod smtp;
mod wizard;

use std::{
    io::{IsTerminal, stdin},
    path::PathBuf,
};

use anyhow::Result;
use clap::{CommandFactory, Parser};
use pimalaya_cli::{error::ErrorReport, log::Logger, printer::Printer, printer::StdoutPrinter};
use pimalaya_config::toml::TomlConfig;

use crate::{cli::Cli, config::Config};

fn main() {
    let cli = Cli::parse();
    let mut printer = StdoutPrinter::new(&cli.json);
    let result = execute(cli, &mut printer);
    ErrorReport::eval(&mut printer, result);
}

fn execute(cli: Cli, printer: &mut StdoutPrinter) -> Result<()> {
    Logger::try_init(&cli.log)?;
    let config = cli.config.paths.as_ref();
    let account = cli.account.name.as_deref();
    let backend = cli.backend;

    let Some(cmd) = cli.cmd else {
        return meet_bare_invocation(printer, config, account.is_some());
    };

    cmd.execute(printer, config, account, backend)
}

/// Meets a bare `himalaya`, which is where a newcomer lands.
///
/// With no command there is nothing to run: a missing configuration
/// raises the offer, and an existing one gets the help, which is also
/// what a script or a JSON caller gets since neither can answer a
/// prompt. A file that exists but fails to parse counts as a
/// configuration, so the offer never proposes to write over a broken
/// one: the parse error surfaces when a real command reads it.
///
/// `--account` names an account to act on, so with no subcommand it is a
/// half-typed command rather than a first run: it gets the help, which
/// points at the commands, instead of an offer to create an account.
fn meet_bare_invocation(
    printer: &mut StdoutPrinter,
    config_paths: &[PathBuf],
    named_account: bool,
) -> Result<()> {
    let configured = Config::from_paths_or_default(config_paths)
        .ok()
        .flatten()
        .is_some();

    if !configured && !named_account && !printer.is_json() && stdin().is_terminal() {
        let path = Config::target_path(config_paths)?;

        // NOTE: a bare invocation has nothing to run after the offer, so
        // a declined one falls back to the help. The wizard already says
        // what to run next when it ran.
        if cli::offer_configuration(printer, config_paths, &path)? {
            return Ok(());
        }
    }

    Cli::command().print_help()?;

    Ok(())
}
