pub mod list;
pub mod thread;
pub mod watch;

use clap::Subcommand;
use color_eyre::Result;

use crate::{config::TomlConfig, printer::Printer};

use self::{
    list::ListEnvelopesCommand, thread::ThreadEnvelopesCommand, watch::WatchEnvelopesCommand,
};

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

    #[command()]
    Watch(WatchEnvelopesCommand),
}

impl EnvelopeSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, config).await,
            Self::Thread(cmd) => cmd.execute(printer, config).await,
            Self::Watch(cmd) => cmd.execute(printer, config).await,
        }
    }
}
