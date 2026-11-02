//! # Repl command
//!
//! The `repl` subcommand, a session over stdin and stdout running the
//! CLI's own commands against one account, so each client connects once
//! rather than once per command.
//!
//! Each input line is split as a shell splits it, parsed with the one-shot
//! grammar and run over the [`Session`] the loop keeps. Standard input
//! being the command stream, a command that would read its own input there
//! is refused ([`check_stdin`]).

use std::{
    io::{self, IsTerminal, Write},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Result, anyhow};
use clap::Parser;
use pimalaya_cli::{
    error::{CodedError, ErrorReport},
    printer::Printer,
};

use crate::{
    cli::{Command, resolve_account},
    error::ErrorCode,
    session::Session,
};

/// Whether a session reads its commands off standard input.
static IN_SESSION: AtomicBool = AtomicBool::new(false);

/// Start a session running commands over one account.
///
/// Each line is a command without the binary name (`envelope list`,
/// `message read 42`), run over clients connected once for the whole
/// session; the account, `--backend` and `--json` are the session's.
/// `quit`, `exit`, or EOF ends it.
#[derive(Debug, Parser)]
pub struct ReplCommand;

/// One parsed input line, the CLI grammar minus the binary name and the
/// global flags.
#[derive(Debug, Parser)]
#[command(no_binary_name = true, infer_subcommands = true)]
struct ReplLine {
    #[command(subcommand)]
    cmd: Command,
}

impl ReplCommand {
    /// Runs the read-eval-print loop until EOF or a quit command, keeping
    /// the session's clients across lines.
    pub fn execute(self, printer: &mut impl Printer, session: &mut Session) -> Result<()> {
        let account = session.account;

        // NOTE: resolved once up front, so a missing configuration or
        // account fails the session rather than its first command.
        resolve_account(printer, account.config_paths, account.name)?;

        claim_stdin();

        let stdin = io::stdin();
        let interactive = stdin.is_terminal();
        let mut line = String::new();

        loop {
            if interactive {
                eprint!("himalaya> ");
                io::stderr().flush()?;
            }

            line.clear();

            if stdin.read_line(&mut line)? == 0 {
                break;
            }

            let input = line.trim();

            if input.is_empty() {
                continue;
            }

            if matches!(input, "quit" | "exit") {
                break;
            }

            let result = match shlex::split(input).map(ReplLine::try_parse_from) {
                None => Err(anyhow!("Cannot split the line: a quote is not closed")),
                Some(Ok(ReplLine {
                    cmd: Command::Repl(_) | Command::Mcp(_),
                })) => Err(anyhow!("Cannot start a session inside a session")),
                #[cfg(feature = "wizard")]
                Some(Ok(ReplLine {
                    cmd: Command::Configure(_),
                })) => Err(anyhow!(
                    "Cannot run the wizard inside a session, run `himalaya configure` instead"
                )),
                Some(Ok(ReplLine { cmd })) => cmd.execute(printer, session),
                // NOTE: help and version are not failures, clap prints
                // them on stdout already, and a terminal gets clap's own
                // rendering of a parse error.
                Some(Err(err)) if !err.use_stderr() || interactive => {
                    let _ = err.print();
                    continue;
                }
                Some(Err(err)) => Err(err.into()),
            };

            if let Err(err) = result {
                printer.out(ErrorReport::<ErrorCode>::from(err))?;
            }

            // NOTE: a text output carries no trailing newline, so one-shot
            // output pipes cleanly, while a JSON value ends with its own.
            if !printer.is_json() {
                println!();
            }

            io::stdout().flush()?;
        }

        Ok(())
    }
}

/// Marks standard input as the session's own, read by the session loop
/// rather than by the commands it runs.
pub fn claim_stdin() {
    IN_SESSION.store(true, Ordering::Relaxed);
}

/// Refuses to read a command's input off standard input inside a session,
/// where it carries the commands that follow.
pub fn check_stdin() -> Result<()> {
    if IN_SESSION.load(Ordering::Relaxed) {
        let message = "Cannot read input from stdin inside a session, \
                       pass it as an argument or a file path";
        return Err(CodedError::new(ErrorCode::StdinTaken, message).into());
    }

    Ok(())
}

/// Whether the commands run inside a session.
pub fn in_session() -> bool {
    IN_SESSION.load(Ordering::Relaxed)
}
