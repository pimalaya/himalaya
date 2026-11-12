//! # Himalaya
//!
//! A CLI to manage emails, and the crate architecture (header-001): the
//! behavioural contract lives in cairn/spec/, the manual provider test
//! reports in cairn/spec/testing/, the history in cairn/log/, the shared
//! conventions in the [Pimalaya ARCHITECTURE][pimalaya].
//!
//! [pimalaya]: https://github.com/pimalaya/.github/blob/master/ARCHITECTURE.md
//!
//! ## Place in the stack
//!
//! An application, the top of the Pimalaya stack: it writes no protocol
//! and no storage logic of its own and ships no library target, only this
//! binary. A thin shell driving the sans-I/O io-* libraries below it,
//! consuming their blocking `*Std` clients and rendering the results.
//!
//! ## Backends and plumbing
//!
//! The network backends are io-imap, io-jmap, io-gmail, io-msgraph,
//! io-smtp and io-managesieve, the local ones io-maildir, io-m2dir, io-mbox
//! and io-pimdir.
//!
//! The CLI plumbing, TOML loading and blocking stream runtime come from
//! pimalaya-cli, pimalaya-config and pimalaya-stream. Every backend sits
//! behind its own cargo feature, so a build ships only the protocols it
//! needs.
//!
//! ## Command families
//!
//! The command tree ([`cli`], `Command`) splits in three. The unified API
//! (mailbox, envelope, flag, message, attachment, vacation) is the
//! cross-protocol surface: the union of what the backends serve, each verb
//! meaning one thing wherever it runs, a backend that cannot give that
//! result declaring it unserved ([`capability`]).
//!
//! The protocol-specific APIs (imap, jmap, gmail, msgraph, maildir,
//! m2dir, mbox, pimdir, smtp, sieve) each expose the full surface of one
//! backend, including operations the unified API cannot model. The meta
//! commands cover the accounts, the sessions (repl, mcp), completions, man
//! pages and JSON Schemas.
//!
//! ## Unified commands and backend selection
//!
//! The unified commands run over a [`shared::client`] `EmailClient` owning
//! one `BackendClient` variant per compiled-in backend: the first
//! configured storage backend the global `--backend` flag allows, local
//! before network, plus an SMTP transport for those that cannot send.
//! The [`capability`] resolver picks them from the account's blocks and
//! refuses, before connecting, a verb the resolved backend does not
//! serve, under `capability-missing`; its tables also write each unified
//! command's `--help` footer and filter the MCP tool list.
//!
//! Each shared method matches the active backend and calls its
//! per-protocol adapter, which converts io-* results into the CLI's own
//! [`email`] shared types. The active [`account`] context is threaded as
//! a sibling argument through every `execute` chain.
//!
//! ## Protocol-specific commands
//!
//! Each protocol module builds its client via a `build_<proto>_client`
//! helper and a `<Proto>Client` wrapper that derefs onto the io-* `*Std`
//! client, ignoring `--backend`. Subcommands are clap-derived structs
//! with an `execute` method the module's command enum dispatches to.
//!
//! The imap command mirrors IMAP's flat command list, gmail and msgraph
//! track their REST resource domains one-to-one, and the filesystem
//! backends expose only what maps to their on-disk layout, leaving MIME
//! rendering to the unified commands.
//!
//! ## Sessions
//!
//! Every command runs over a [`session`] `Session`, which builds the client
//! the command needs on first use and keeps it, dropping it when a command
//! fails on it. A one-shot run uses a fresh session for its single command;
//! the [`repl`] keeps one across the lines of its stdin, so each client
//! connects once for the whole session. The [`mcp`] server keeps one the
//! same way, each tool call parsed back into a unified API command. Both
//! run on pimalaya-cli's shared core (its `repl` loop, its `mcp` protocol
//! answers and tool grammar), which claims stdin, so a command reading
//! its input there is refused under `stdin-taken`.
//!
//! ## Configuration and output
//!
//! Config is loaded by pimalaya-config from every existing canonical path
//! (the rc file last), or the `-c` paths, one per flag, later paths
//! deep-merged on top. The schema ([`config`]) is multi-account: a
//! top-level block plus named account blocks carrying optional
//! per-backend sub-blocks. pimalaya-config also takes the account `-a`
//! names or the default one, its typed errors naming what is missing.
//!
//! Himalaya never writes a configuration: a person writes it from the
//! documented sample. A person running Himalaya without one meets the
//! welcome ([`cli::welcome`], pimalaya-cli's in Himalaya's words), which
//! creates it from the sample shipped beside the binary or downloads it;
//! a script or a JSON caller gets the error instead. A bare `himalaya`
//! prints the help when a configuration exists.
//!
//! Output follows the Pimalaya rule: data and errors go to stdout through
//! the printer, `--json` switching every command to JSON, and stderr
//! carries the logs, the welcome, the REPL's prompt and its help in a
//! JSON session, and the failures of the MCP server, whose stdout is the
//! protocol's. Text output is plain text, with no border, no colour and
//! nothing sized to the terminal: a list prints one item per line as
//! aligned columns between the fields placing it (the account above, the
//! page under), and a record as aligned `Label: value` lines. Each
//! command's doc comment is its `--help`, so `himalaya <command> --help`
//! is the canonical usage reference.

mod account;
mod backend;
mod capability;
mod cli;
mod config;
#[cfg(any(backend, feature = "smtp"))]
mod email;
mod error;
#[cfg(feature = "gmail")]
mod gmail;
#[cfg(feature = "imap")]
mod imap;
#[cfg(feature = "jmap")]
mod jmap;
mod json_schema;
mod listing;
#[cfg(feature = "m2dir")]
mod m2dir;
#[cfg(feature = "maildir")]
mod maildir;
#[cfg(feature = "mbox")]
mod mbox;
mod mcp;
#[cfg(feature = "msgraph")]
mod msgraph;
#[cfg(feature = "pimdir")]
mod pimdir;
mod repl;
mod session;
mod shared;
#[cfg(feature = "sieve")]
mod sieve;
#[cfg(feature = "smtp")]
mod smtp;

use std::process;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{
    clap::args::LogFlags,
    error::{EXIT_FAILURE, ErrorReport},
    log::Logger,
    printer::StdoutPrinter,
    welcome::{is_wanted, shipped_sample},
};
use pimalaya_config::toml::ConfigMissingError;
use simd_cpu::SimdCpu;

use crate::{
    cli::{Cli, Command},
    error::ErrorCode,
    session::{Session, SessionAccount},
};

fn main() {
    // NOTE: the base64 and quoted-printable codecs read the installed CPU
    // token, the portable baseline until one is installed.
    let _ = SimdCpu::install_detected();

    let cli = Cli::parse();
    let mut printer = StdoutPrinter::new(&cli.json);

    let account = SessionAccount {
        config_paths: &cli.config.paths,
        name: cli.account.name.as_deref(),
        backend: cli.backend,
    };

    let result = execute(cli.cmd, &cli.log, account, &mut printer);

    // A person missing a configuration meets the welcome, a script or a
    // JSON caller the error.
    if let Err(err) = &result
        && let Some(missing) = err.downcast_ref::<ConfigMissingError>()
        && is_wanted(&printer)
    {
        let sample = shipped_sample("himalaya");
        let welcome = cli::welcome(&missing.path, account.config_paths, sample.as_deref());
        eprint!("{welcome}");
        process::exit(EXIT_FAILURE);
    }

    ErrorReport::<ErrorCode>::eval_coded(&mut printer, result);
}

/// Runs the subcommand over a fresh session, or meets a bare `himalaya`.
fn execute(
    cmd: Option<Command>,
    log: &LogFlags,
    account: SessionAccount,
    printer: &mut StdoutPrinter,
) -> Result<()> {
    Logger::try_init(log)?;

    let Some(cmd) = cmd else {
        return cli::meet_bare_invocation(account.config_paths);
    };

    #[cfg_attr(
        not(any(backend, feature = "smtp", feature = "sieve")),
        allow(clippy::needless_update)
    )]
    let mut session = Session {
        account,
        ..Default::default()
    };

    cmd.execute(printer, &mut session)
}
