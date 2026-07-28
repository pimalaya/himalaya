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
//! io-smtp; the local storage backends are io-maildir and io-m2dir.
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
//! APIs (imap, jmap, gmail, msgraph, maildir, m2dir, smtp) each expose
//! the full surface of one backend, including operations the shared API
//! cannot model. The meta commands (account, completion, manual) cover
//! account configuration, shell completions and man pages.
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
mod shared;
#[cfg(feature = "smtp")]
mod smtp;
mod wizard;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use pimalaya_cli::{error::ErrorReport, log::Logger, printer::StdoutPrinter};

use crate::{cli::Cli, wizard::discover};

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

    match cli.cmd {
        Some(cmd) => cmd.execute(printer, config, account, backend),
        // A bare `himalaya` runs the first-run wizard; but `--account`
        // names an account to act on, so with no subcommand it is a
        // half-typed command — show the help to point at the commands
        // rather than dropping into account creation.
        None if account.is_some() => {
            Cli::command().print_help()?;
            Ok(())
        }
        None => discover::run(printer),
    }
}
