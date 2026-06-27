use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::msgraph::{
    client::MsgraphClient,
    message::{
        copy::MsgraphMessageCopyCommand, create::MsgraphMessageCreateCommand,
        delete::MsgraphMessageDeleteCommand, get::MsgraphMessageGetCommand,
        list::MsgraphMessageListCommand, r#move::MsgraphMessageMoveCommand,
        send::MsgraphMessageSendCommand, update::MsgraphMessageUpdateCommand,
    },
};

/// Manage Microsoft Graph messages (`me.messages`).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MsgraphMessageCommand {
    List(MsgraphMessageListCommand),
    Get(MsgraphMessageGetCommand),
    Create(MsgraphMessageCreateCommand),
    Update(MsgraphMessageUpdateCommand),
    Send(MsgraphMessageSendCommand),
    Copy(MsgraphMessageCopyCommand),
    #[command(name = "move")]
    Move(MsgraphMessageMoveCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(MsgraphMessageDeleteCommand),
}

impl MsgraphMessageCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Send(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}
