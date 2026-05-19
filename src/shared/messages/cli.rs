use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::shared::{
    client::EmailClient,
    messages::{
        add::MessageAddCommand, compose::MessageComposeCommand,
        compose_with::MessageComposeWithCommand, copy::MessageCopyCommand,
        forward::MessageForwardCommand, forward_with::MessageForwardWithCommand,
        mailto::MessageMailtoCommand, mv::MessageMoveCommand, read::MessageReadCommand,
        read_with::MessageReadWithCommand, reply::MessageReplyCommand,
        reply_with::MessageReplyWithCommand, send::MessageSendCommand,
    },
};

/// Shared API to manage messages for the active account.
///
/// A message is composed of headers (key-value properties) and a body
/// (suite of MIME parts). The built-in `compose` / `reply` / `forward`
/// / `read` subcommands cover simple cases via CLI flags. For
/// non-default workflows, the `-with` variants delegate composition
/// and rendering to a user-defined command from
/// `[message.composer.*]` / `[message.reader.*]`.
#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    #[command(visible_alias = "save")]
    Add(MessageAddCommand),
    #[command(visible_alias = "write", alias = "new")]
    Compose(MessageComposeCommand),
    #[command(visible_alias = "write-with")]
    ComposeWith(MessageComposeWithCommand),
    #[command(visible_alias = "cp")]
    Copy(MessageCopyCommand),
    #[command(visible_alias = "fwd")]
    Forward(MessageForwardCommand),
    ForwardWith(MessageForwardWithCommand),
    Mailto(MessageMailtoCommand),
    #[command(visible_alias = "mv")]
    Move(MessageMoveCommand),
    Read(MessageReadCommand),
    ReadWith(MessageReadWithCommand),
    Reply(MessageReplyCommand),
    ReplyWith(MessageReplyWithCommand),
    Send(MessageSendCommand),
}

impl MessageCommand {
    pub fn execute(self, printer: &mut impl Printer, client: EmailClient) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, client),
            Self::Compose(cmd) => cmd.execute(printer, client),
            Self::ComposeWith(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Forward(cmd) => cmd.execute(printer, client),
            Self::ForwardWith(cmd) => cmd.execute(printer, client),
            Self::Mailto(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),
            Self::Read(cmd) => cmd.execute(printer, client),
            Self::ReadWith(cmd) => cmd.execute(printer, client),
            Self::Reply(cmd) => cmd.execute(printer, client),
            Self::ReplyWith(cmd) => cmd.execute(printer, client),
            Self::Send(cmd) => cmd.execute(printer, client),
        }
    }
}
