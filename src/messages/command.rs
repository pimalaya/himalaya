use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
    messages::{
        add::MessagesAddCommand, compose::MessagesComposeCommand, copy::MessagesCopyCommand,
        get::MessagesGetCommand, mv::MessagesMoveCommand, send::MessagesSendCommand,
    },
};

/// Manage messages through whichever backend the active account has
/// configured.
///
/// The active backend is selected by `--backend` (defaults to `auto`,
/// which picks the first configured backend in priority order). Note
/// that `messages send` only has SMTP and JMAP arms; the others have
/// IMAP, JMAP and Maildir arms.
#[derive(Debug, Subcommand)]
pub enum MessagesCommand {
    Add(MessagesAddCommand),
    Compose(MessagesComposeCommand),
    Copy(MessagesCopyCommand),
    Get(MessagesGetCommand),
    #[command(name = "move")]
    Move(MessagesMoveCommand),
    Send(MessagesSendCommand),
}

impl MessagesCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, config, account_config, backend),
            Self::Compose(cmd) => cmd.execute(printer, config, account_config, backend),
            Self::Copy(cmd) => cmd.execute(printer, config, account_config, backend),
            Self::Get(cmd) => cmd.execute(printer, config, account_config, backend),
            Self::Move(cmd) => cmd.execute(printer, config, account_config, backend),
            Self::Send(cmd) => cmd.execute(printer, config, account_config, backend),
        }
    }
}
