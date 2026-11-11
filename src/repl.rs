//! # Repl command
//!
//! The `repl` subcommand, a session over stdin and stdout running the
//! CLI's own commands against one account, so each client connects once
//! rather than once per command.
//!
//! pimalaya-cli's [`Repl`] runs the loop: each line split as a shell splits
//! it, parsed with the one-shot grammar plus `--json` and run over the
//! [`Session`] kept here, every answer on stdout, the session flags refused
//! by name. It claims stdin, so a command that would read its own input
//! there is refused (`pimalaya_cli::session::check_stdin`).

use anyhow::{Result, anyhow};
use clap::Parser;
use pimalaya_cli::{printer::Printer, repl::Repl};

use crate::{
    cli::{Command, resolve_account},
    error::ErrorCode,
    session::Session,
};

/// Start a session running commands over one account.
///
/// Each line is a command without the binary name (`envelope list`,
/// `message read 42`), run over clients connected once for the whole
/// session; the account and `--backend` are the session's, and `--json`
/// on a line switches that line alone to JSON. `quit`, `exit`, or EOF
/// ends it.
#[derive(Debug, Parser)]
pub struct ReplCommand;

impl ReplCommand {
    /// Runs the read-eval-print loop until EOF or a quit command, keeping
    /// the session's clients across lines, each line printing as
    /// `printer` does unless it asks for JSON.
    pub fn execute(self, printer: &impl Printer, session: &mut Session) -> Result<()> {
        let account = session.account;

        // Resolved once up front, so a missing configuration or account
        // fails the session rather than its first command.
        resolve_account(account.config_paths, account.name)?;

        let mut repl = Repl::new("himalaya", printer.is_json());
        repl.fixed.extend(["-b", "--backend"]);

        repl.run::<Command, ErrorCode>(|printer, cmd| match cmd {
            Command::Repl(_) | Command::Mcp(_) => {
                Err(anyhow!("Cannot start a session inside a session"))
            }
            cmd => cmd.execute(printer, session),
        })
    }
}
