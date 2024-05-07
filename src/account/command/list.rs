use clap::Parser;
use color_eyre::Result;
use tracing::info;

use crate::{account::Accounts, config::TomlConfig, printer::Printer};

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
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing list accounts command");

        let accounts: Accounts = config.accounts.iter().into();

        printer.print_table(accounts, self.table_max_width)?;
        Ok(())
    }
}
