#[cfg(feature = "message-copy")]
pub mod copy;
#[cfg(feature = "message-delete")]
pub mod delete;
#[cfg(feature = "message-forward")]
pub mod forward;
#[cfg(feature = "message-mailto")]
pub mod mailto;
#[cfg(feature = "message-move")]
pub mod move_;
#[cfg(feature = "message-read")]
pub mod read;
#[cfg(feature = "message-reply")]
pub mod reply;
#[cfg(feature = "message-save")]
pub mod save;
#[cfg(feature = "message-send")]
pub mod send;
#[cfg(feature = "message-write")]
pub mod write;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

#[cfg(feature = "message-copy")]
use self::copy::MessageCopyCommand;
#[cfg(feature = "message-delete")]
use self::delete::MessageDeleteCommand;
#[cfg(feature = "message-forward")]
use self::forward::MessageForwardCommand;
#[cfg(feature = "message-mailto")]
use self::mailto::MessageMailtoCommand;
#[cfg(feature = "message-move")]
use self::move_::MessageMoveCommand;
#[cfg(feature = "message-read")]
use self::read::MessageReadCommand;
#[cfg(feature = "message-reply")]
use self::reply::MessageReplyCommand;
#[cfg(feature = "message-save")]
use self::save::MessageSaveCommand;
#[cfg(feature = "message-send")]
use self::send::MessageSendCommand;
#[cfg(feature = "message-write")]
use self::write::MessageWriteCommand;

/// Manage messages.
///
/// A message is the content of an email. It is composed of headers
/// (located at the top of the message) and a body (located at the
/// bottom of the message). Both are separated by two new lines. This
/// subcommand allows you to manage them.
#[derive(Debug, Subcommand)]
pub enum MessageSubcommand {
    #[cfg(feature = "message-read")]
    #[command(arg_required_else_help = true)]
    Read(MessageReadCommand),

    #[cfg(feature = "message-write")]
    #[command(aliases = ["add", "create", "new", "compose"])]
    Write(MessageWriteCommand),

    #[cfg(feature = "message-reply")]
    #[command()]
    Reply(MessageReplyCommand),

    #[cfg(feature = "message-forward")]
    #[command(aliases = ["fwd", "fd"])]
    Forward(MessageForwardCommand),

    #[cfg(feature = "message-mailto")]
    #[command()]
    Mailto(MessageMailtoCommand),

    #[cfg(feature = "message-save")]
    #[command(arg_required_else_help = true)]
    Save(MessageSaveCommand),

    #[cfg(feature = "message-send")]
    #[command(arg_required_else_help = true)]
    Send(MessageSendCommand),

    #[cfg(feature = "message-copy")]
    #[command(arg_required_else_help = true)]
    #[command(aliases = ["cpy", "cp"])]
    Copy(MessageCopyCommand),

    #[cfg(feature = "message-move")]
    #[command(arg_required_else_help = true)]
    #[command(alias = "mv")]
    Move(MessageMoveCommand),

    #[cfg(feature = "message-delete")]
    #[command(arg_required_else_help = true)]
    #[command(aliases = ["remove", "rm"])]
    Delete(MessageDeleteCommand),
}

impl MessageSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            #[cfg(feature = "message-read")]
            Self::Read(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "message-write")]
            Self::Write(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "message-reply")]
            Self::Reply(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "message-forward")]
            Self::Forward(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "message-mailto")]
            Self::Mailto(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "message-save")]
            Self::Save(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "message-send")]
            Self::Send(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "message-copy")]
            Self::Copy(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "message-move")]
            Self::Move(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "message-delete")]
            Self::Delete(cmd) => cmd.execute(printer, config).await,
        }
    }
}
