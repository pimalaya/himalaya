use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    folder::arg::name::FolderNameOptionalArg,
    printer::{PrintTableOpts, Printer},
    ui::arg::max_width::TableMaxWidthFlag,
};

/// List all envelopes.
///
/// This command allows you to list all envelopes included in the
/// given folder.
#[derive(Debug, Parser)]
pub struct ListEnvelopesCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalArg,

    /// The page number.
    ///
    /// The page number starts from 1 (which is the default). Giving a
    /// page number to big will result in a out of bound error.
    #[arg(long, short, value_name = "NUMBER", default_value = "1")]
    pub page: usize,

    /// The page size.
    ///
    /// Determine the amount of envelopes a page should contain.
    #[arg(long, short = 's', value_name = "NUMBER")]
    pub page_size: Option<usize>,

    #[command(flatten)]
    pub table: TableMaxWidthFlag,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl ListEnvelopesCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing envelope list command");

        let folder = &self.folder.name;

        let some_account_name = self.account.name.as_ref().map(String::as_str);
        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(some_account_name, self.cache.disable)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;

        let page_size = self
            .page_size
            .unwrap_or(account_config.get_envelope_list_page_size());
        let page = 1.max(self.page) - 1;

        let envelopes = backend.list_envelopes(folder, page_size, page).await?;

        printer.print_table(
            Box::new(envelopes),
            PrintTableOpts {
                format: &account_config.get_message_read_format(),
                max_width: self.table.max_width,
            },
        )?;

        Ok(())
    }
}
