use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
    email_client::build,
    flags::arg::{FlagsArg, MailboxFlag, MessageIdsArg},
};

/// Add flag(s) to message(s) for the active account.
#[derive(Debug, Parser)]
pub struct FlagsAddCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,
    #[command(flatten)]
    pub flags: FlagsArg,
    #[command(flatten)]
    pub mailbox: MailboxFlag,
}

impl FlagsAddCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        let mut ctx = build(config, account_config, backend)?;

        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();
        let flags: Vec<io_email::flag::Flag> = self.flags.inner.iter().map(Into::into).collect();

        ctx.client.add_flags(&self.mailbox.inner, &ids, &flags)?;

        printer.out(Message::new("Flag(s) successfully added"))
    }
}
