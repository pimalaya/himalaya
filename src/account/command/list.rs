use clap::Parser;
use color_eyre::Result;
use tracing::info;

use crate::{
    account::{Accounts, AccountsTable},
    config::Config,
    printer::Printer,
};

/// List all accounts.
///
/// This command lists all accounts defined in your TOML configuration
/// file.
#[derive(Debug, Parser)]
pub struct AccountListCommand {
    /// The maximum width the table should not exceed.
    ///
    /// This argument will force the table not to exceed the given
    /// width in pixels. Columns may shrink with ellipsis in order to
    /// fit the width.
    #[arg(long, short = 'w', name = "table_max_width", value_name = "PIXELS")]
    pub table_max_width: Option<u16>,
}

impl AccountListCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &Config) -> Result<()> {
        info!("executing list accounts command");

        let accounts = Accounts::from(config.accounts.iter());
        let table = AccountsTable::from(accounts)
            .with_some_width(self.table_max_width)
            .with_some_preset(config.account_list_table_preset())
            .with_some_name_color(config.account_list_table_name_color())
            .with_some_backends_color(config.account_list_table_backends_color())
            .with_some_default_color(config.account_list_table_default_color());

        printer.out(table)?;
        Ok(())
    }
}
