use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    cli::BackendArg,
    config::{AccountConfig, Config},
    email_client::build,
    envelopes::table::EnvelopesTable,
};

/// List envelopes for the active account, regardless of the underlying
/// backend (IMAP, JMAP or Maildir).
#[derive(Debug, Parser)]
pub struct EnvelopesListCommand {
    /// Path or name of the IMAP/Maildir mailbox.
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "PATH",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    /// Page number, starting from 1. The most recent envelopes are on
    /// page 1.
    #[arg(long, short = 'p', value_name = "N", default_value = "1")]
    pub page: u32,

    /// Maximum number of envelopes per page.
    #[arg(
        long = "page-size",
        short = 's',
        value_name = "N",
        default_value = "25"
    )]
    pub page_size: u32,
}

impl EnvelopesListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config: Config,
        account_config: AccountConfig,
        backend: BackendArg,
    ) -> Result<()> {
        let mut ctx = build(config, account_config, backend)?;
        let envelopes =
            ctx.client
                .list_envelopes(&self.mailbox, Some(self.page), Some(self.page_size))?;

        printer.out(EnvelopesTable {
            preset: ctx.table_preset,
            arrangement: ctx.table_arrangement,
            envelopes,
        })
    }
}
