pub mod list;
pub mod thread;

use clap::Subcommand;
use color_eyre::Result;

use crate::{config::Config, printer::Printer};

use self::{list::ListEnvelopesCommand, thread::ThreadEnvelopesCommand};

/// Manage envelopes.
///
/// An envelope is a small representation of a message. It contains an
/// identifier (given by the backend), some flags as well as few
/// headers from the message itself. This subcommand allows you to
/// manage them.
#[derive(Debug, Subcommand)]
pub enum EnvelopeSubcommand {
    #[command(alias = "lst")]
    List(ListEnvelopesCommand),

    #[command()]
    Thread(ThreadEnvelopesCommand),
}

impl EnvelopeSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &Config) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, config).await,
            Self::Thread(cmd) => cmd.execute(printer, config).await,
        }
    }
}
