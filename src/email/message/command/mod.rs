pub mod copy;
pub mod delete;
pub mod forward;
pub mod mailto;
pub mod r#move;
pub mod read;
pub mod reply;
pub mod save;
pub mod send;
pub mod write;

use color_eyre::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::{
    copy::MessageCopyCommand, delete::MessageDeleteCommand, forward::MessageForwardCommand,
    mailto::MessageMailtoCommand, r#move::MessageMoveCommand, read::MessageReadCommand,
    reply::MessageReplyCommand, save::MessageSaveCommand, send::MessageSendCommand,
    write::MessageWriteCommand,
};

/// Manage messages.
///
/// A message is the content of an email. It is composed of headers
/// (located at the top of the message) and a body (located at the
/// bottom of the message). Both are separated by two new lines. This
/// subcommand allows you to manage them.
#[derive(Debug, Subcommand)]
pub enum MessageSubcommand {
    #[command(arg_required_else_help = true)]
    Read(MessageReadCommand),

    #[command(aliases = ["add", "create", "new", "compose"])]
    Write(MessageWriteCommand),

    #[command()]
    Reply(MessageReplyCommand),

    #[command(aliases = ["fwd", "fd"])]
    Forward(MessageForwardCommand),

    #[command()]
    Mailto(MessageMailtoCommand),

    Save(MessageSaveCommand),

    Send(MessageSendCommand),

    #[command(arg_required_else_help = true)]
    #[command(aliases = ["cpy", "cp"])]
    Copy(MessageCopyCommand),

    #[command(arg_required_else_help = true)]
    #[command(alias = "mv")]
    Move(MessageMoveCommand),

    #[command(arg_required_else_help = true)]
    #[command(aliases = ["remove", "rm"])]
    Delete(MessageDeleteCommand),
}

impl MessageSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Read(cmd) => cmd.execute(printer, config).await,
            Self::Write(cmd) => cmd.execute(printer, config).await,
            Self::Reply(cmd) => cmd.execute(printer, config).await,
            Self::Forward(cmd) => cmd.execute(printer, config).await,
            Self::Mailto(cmd) => cmd.execute(printer, config).await,
            Self::Save(cmd) => cmd.execute(printer, config).await,
            Self::Send(cmd) => cmd.execute(printer, config).await,
            Self::Copy(cmd) => cmd.execute(printer, config).await,
            Self::Move(cmd) => cmd.execute(printer, config).await,
            Self::Delete(cmd) => cmd.execute(printer, config).await,
        }
    }
}
