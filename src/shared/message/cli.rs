//! # Message command
//!
//! The `message` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    capability::{self, Capability},
    shared::{
        client::EmailClient,
        message::{
            add::MessageAddCommand, compose::MessageComposeCommand, copy::MessageCopyCommand,
            delete::MessageDeleteCommand, forward::MessageForwardCommand, mv::MessageMoveCommand,
            parse::MessageParseCommand, read::MessageReadCommand, reply::MessageReplyCommand,
            send::MessageSendCommand,
        },
    },
};

/// Manage messages using the shared API.
///
/// A message is headers plus a body of MIME parts. The `compose`, `reply`,
/// `forward` and `read` subcommands cover the simple cases through flags.
///
/// Richer composition belongs to a standalone tool such as mml, piped into
/// `message send` or `message add`.
#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    #[command(visible_alias = "save", after_long_help = capability::help("message", "add"))]
    Add(MessageAddCommand),
    #[command(visible_alias = "write", alias = "new")]
    Compose(MessageComposeCommand),
    #[command(visible_alias = "cp", after_long_help = capability::help("message", "copy"))]
    Copy(MessageCopyCommand),
    #[command(visible_alias = "rm", alias = "remove", after_long_help = capability::help("message", "delete"))]
    Delete(MessageDeleteCommand),
    #[command(visible_alias = "fwd", after_long_help = capability::help("message", "forward"))]
    Forward(MessageForwardCommand),
    #[command(visible_alias = "mv", after_long_help = capability::help("message", "move"))]
    Move(MessageMoveCommand),
    Parse(MessageParseCommand),
    #[command(after_long_help = capability::help("message", "read"))]
    Read(MessageReadCommand),
    #[command(after_long_help = capability::help("message", "reply"))]
    Reply(MessageReplyCommand),
    #[command(after_long_help = capability::help("message", "send"))]
    Send(MessageSendCommand),
}

impl MessageCommand {
    /// The capabilities the subcommand reaches with the arguments it was
    /// given, checked before any connection.
    ///
    /// A copy `message.send.save-copy` configures is checked once the
    /// account is read, still before the send.
    pub fn capabilities(&self) -> Vec<Capability> {
        let composed = |read: bool, save: bool, send: bool| {
            [
                (read, Capability::MessageRead),
                (save, Capability::MessageAdd),
                (send, Capability::Submit),
            ]
            .into_iter()
            .filter_map(|(reached, capability)| reached.then_some(capability))
            .collect()
        };

        match self {
            Self::Add(cmd) => composed(false, true, cmd.send),
            Self::Compose(cmd) => composed(false, cmd.save.is_some(), cmd.send),
            Self::Copy(_) => vec![Capability::MessageCopy],
            Self::Delete(_) => vec![Capability::MessageTrash],
            Self::Forward(cmd) => composed(true, cmd.save.is_some(), cmd.send),
            Self::Move(_) => vec![Capability::MessageMove],
            Self::Parse(_) => Vec::new(),
            Self::Read(cmd) if cmd.seen => vec![Capability::MessageRead, Capability::FlagsSeen],
            Self::Read(_) => vec![Capability::MessageRead],
            Self::Reply(cmd) => composed(true, cmd.save.is_some(), cmd.send),
            Self::Send(cmd) => composed(false, cmd.save.is_some(), true),
        }
    }

    /// Runs the subcommand against the active account's client.
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
            Self::Delete(cmd) => cmd.execute(printer, account, client),
            Self::Forward(cmd) => cmd.execute(printer, account, client),
            Self::Move(cmd) => cmd.execute(printer, account, client),
            Self::Parse(cmd) => cmd.execute(printer),
            Self::Read(cmd) => cmd.execute(printer, account, client),
            Self::Reply(cmd) => cmd.execute(printer, account, client),
            Self::Send(cmd) => cmd.execute(printer, account, client),
        }
    }
}
