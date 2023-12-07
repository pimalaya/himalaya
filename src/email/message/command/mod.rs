pub mod copy;
pub mod delete;
pub mod move_;
pub mod read;
pub mod save;
pub mod send;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::{
    copy::MessageCopyCommand, delete::MessageDeleteCommand, move_::MessageMoveCommand,
    read::MessageReadCommand, save::MessageSaveCommand, send::MessageSendCommand,
};

/// Subcommand to manage messages
#[derive(Debug, Subcommand)]
pub enum MessageSubcommand {
    /// Read a message
    #[command(arg_required_else_help = true)]
    Read(MessageReadCommand),

    /// Save a message to a folder
    #[command(arg_required_else_help = true)]
    #[command(alias = "add", alias = "create")]
    Save(MessageSaveCommand),

    /// Send a message
    #[command(arg_required_else_help = true)]
    Send(MessageSendCommand),

    /// Copy a message from a source folder to a target folder
    #[command(arg_required_else_help = true)]
    Copy(MessageCopyCommand),

    /// Move a message from a source folder to a target folder
    #[command(arg_required_else_help = true)]
    Move(MessageMoveCommand),

    /// Delete a message from a folder
    #[command(arg_required_else_help = true)]
    Delete(MessageDeleteCommand),
}

impl MessageSubcommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Read(cmd) => cmd.execute(printer, config).await,
            Self::Save(cmd) => cmd.execute(printer, config).await,
            Self::Send(cmd) => cmd.execute(printer, config).await,
            Self::Copy(cmd) => cmd.execute(printer, config).await,
            Self::Move(cmd) => cmd.execute(printer, config).await,
            Self::Delete(cmd) => cmd.execute(printer, config).await,
        }
    }
}
