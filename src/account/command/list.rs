use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    account::Accounts,
    config::TomlConfig,
    printer::{PrintTableOpts, Printer},
    ui::arg::max_width::TableMaxWidthFlag,
};

/// List all accounts.
///
/// This command lists all accounts defined in your TOML configuration
/// file.
#[derive(Debug, Parser)]
pub struct AccountListCommand {
    #[command(flatten)]
    pub table: TableMaxWidthFlag,
}

impl AccountListCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing account list command");

        let accounts: Accounts = config.accounts.iter().into();

        printer.print_table(
            Box::new(accounts),
            PrintTableOpts {
                format: &Default::default(),
                max_width: self.table.max_width,
            },
        )
    }
}
