use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
    email_client::build,
    mailboxes::table::MailboxesTable,
};

/// List mailboxes for the active account, regardless of the underlying
/// backend (IMAP, JMAP or Maildir).
#[derive(Debug, Parser)]
pub struct MailboxesListCommand;

impl MailboxesListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        let mut ctx = build(config, account_config, backend)?;
        let mailboxes = ctx.client.list_mailboxes()?;

        printer.out(MailboxesTable {
            preset: ctx.table_preset,
            arrangement: ctx.table_arrangement,
            mailboxes,
        })
    }
}
