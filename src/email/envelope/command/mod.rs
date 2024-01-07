#[cfg(feature = "envelope-list")]
pub mod list;
#[cfg(feature = "envelope-watch")]
pub mod watch;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

#[cfg(feature = "envelope-list")]
use self::list::ListEnvelopesCommand;
#[cfg(feature = "envelope-watch")]
use self::watch::WatchEnvelopesCommand;

/// Manage envelopes.
///
/// An envelope is a small representation of a message. It contains an
/// identifier (given by the backend), some flags as well as few
/// headers from the message itself. This subcommand allows you to
/// manage them.
#[derive(Debug, Subcommand)]
pub enum EnvelopeSubcommand {
    #[cfg(feature = "envelope-list")]
    #[command(alias = "lst")]
    List(ListEnvelopesCommand),

    #[cfg(feature = "envelope-watch")]
    #[command()]
    Watch(WatchEnvelopesCommand),
}

impl EnvelopeSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            #[cfg(feature = "envelope-list")]
            Self::List(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "envelope-watch")]
            Self::Watch(cmd) => cmd.execute(printer, config).await,
        }
    }
}
