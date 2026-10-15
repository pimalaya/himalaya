//! # pimdir flag command
//!
//! The `pimdir flag` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    pimdir::{
        client::PimdirClient,
        flag::{
            add::PimdirFlagAddCommand, list::PimdirFlagListCommand,
            remove::PimdirFlagRemoveCommand, set::PimdirFlagSetCommand,
        },
    },
};

/// Read and stage any flag on a message, keywords included.
///
/// The shared `flag` commands take the four flags every backend carries.
/// A pimdir store carries whatever its sources read, and these name a
/// flag by its raw spelling, as `imap store` does on IMAP. Whether a
/// source can push a keyword is its own declaration: one that cannot
/// refuses the change, one that can in part stages it with a warning.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PimdirFlagCommand {
    List(PimdirFlagListCommand),
    Add(PimdirFlagAddCommand),
    Set(PimdirFlagSetCommand),
    Remove(PimdirFlagRemoveCommand),
}

impl PimdirFlagCommand {
    /// Runs the subcommand against the account's pimdir store.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Add(cmd) => cmd.execute(printer, account, client),
            Self::Set(cmd) => cmd.execute(printer, account, client),
            Self::Remove(cmd) => cmd.execute(printer, account, client),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::PimdirFlagCommand;

    #[derive(Debug, Parser)]
    struct Harness {
        #[command(subcommand)]
        flag: PimdirFlagCommand,
    }

    /// clap checks argument ids only when a command is built, so a clash
    /// between flattened arguments would otherwise surface as a panic on
    /// the first run rather than in the test suite.
    #[test]
    fn the_arguments_are_well_formed() {
        Harness::command().debug_assert();
    }

    #[test]
    fn a_write_command_parses_its_flags_mailbox_and_ids() {
        let harness = Harness::try_parse_from([
            "t",
            "add",
            "-m",
            "imap/INBOX",
            "-f",
            "act",
            "-f",
            "\\Seen",
            "41",
            "42",
        ])
        .unwrap();
        let PimdirFlagCommand::Add(add) = harness.flag else {
            panic!("expected add");
        };
        assert_eq!(add.mailbox.inner.as_deref(), Some("imap/INBOX"));
        assert_eq!(add.flags.inner, ["act", "\\Seen"]);
        assert_eq!(add.message_ids.inner, ["41", "42"]);
    }
}
