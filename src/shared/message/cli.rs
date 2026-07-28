use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

#[cfg(backend)]
use crate::shared::message::{
    add::MessageAddCommand, copy::MessageCopyCommand, delete::MessageDeleteCommand,
    forward::MessageForwardCommand, mv::MessageMoveCommand, read::MessageReadCommand,
    reply::MessageReplyCommand,
};
use crate::{
    account::context::Account,
    shared::{
        client::EmailClient,
        message::{compose::MessageComposeCommand, send::MessageSendCommand},
    },
};

/// Manage messages using the shared API.
///
/// A message is composed of headers (key-value properties) and a body (suite of
/// MIME parts). The built-in `compose` / `reply` / `forward` / `read`
/// subcommands cover simple cases via CLI flags. Richer composition is
/// delegated to standalone tools (e.g.
/// [`mml`](https://github.com/pimalaya/mml)) wired up through shell pipelines
/// into `messages send` / `messages add`.
#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    #[cfg(backend)]
    #[command(visible_alias = "save")]
    Add(MessageAddCommand),
    #[command(visible_alias = "write", alias = "new")]
    Compose(MessageComposeCommand),
    #[cfg(backend)]
    #[command(visible_alias = "cp")]
    Copy(MessageCopyCommand),
    #[cfg(backend)]
    #[command(visible_alias = "rm", alias = "remove")]
    Delete(MessageDeleteCommand),
    #[cfg(backend)]
    #[command(visible_alias = "fwd")]
    Forward(MessageForwardCommand),
    #[cfg(backend)]
    #[command(visible_alias = "mv")]
    Move(MessageMoveCommand),
    #[cfg(backend)]
    Read(MessageReadCommand),
    #[cfg(backend)]
    Reply(MessageReplyCommand),
    Send(MessageSendCommand),
}

impl MessageCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        match self {
            #[cfg(backend)]
            Self::Add(cmd) => cmd.execute(printer, account, client),
            Self::Compose(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Copy(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Delete(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Forward(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Move(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Read(cmd) => cmd.execute(printer, account, client),
            #[cfg(backend)]
            Self::Reply(cmd) => cmd.execute(printer, account, client),
            Self::Send(cmd) => cmd.execute(printer, account, client),
        }
    }
}
