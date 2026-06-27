use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient,
    message::{
        add::MessageAddCommand, compose::MessageComposeCommand, copy::MessageCopyCommand,
        forward::MessageForwardCommand, mv::MessageMoveCommand, read::MessageReadCommand,
        reply::MessageReplyCommand, send::MessageSendCommand,
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
    #[command(visible_alias = "save")]
    Add(MessageAddCommand),
    #[command(visible_alias = "write", alias = "new")]
    Compose(MessageComposeCommand),
    #[command(visible_alias = "cp")]
    Copy(MessageCopyCommand),
    #[command(visible_alias = "fwd")]
    Forward(MessageForwardCommand),
    #[command(visible_alias = "mv")]
    Move(MessageMoveCommand),
    Read(MessageReadCommand),
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
            Self::Add(cmd) => cmd.execute(printer, account, client),
            Self::Compose(cmd) => cmd.execute(printer, account, client),
            Self::Copy(cmd) => cmd.execute(printer, account, client),
            Self::Forward(cmd) => cmd.execute(printer, account, client),
            Self::Move(cmd) => cmd.execute(printer, account, client),
            Self::Read(cmd) => cmd.execute(printer, account, client),
            Self::Reply(cmd) => cmd.execute(printer, account, client),
            Self::Send(cmd) => cmd.execute(printer, account, client),
        }
    }
}
