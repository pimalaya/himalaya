use anyhow::Result;
use clap::Parser;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    cache::arg::disable::DisableCacheFlag,
    config::TomlConfig,
    folder::arg::name::FolderNameOptionalArg,
    printer::{PrintTableOpts, Printer},
    ui::arg::max_width::MaxTableWidthFlag,
};

/// List all envelopes from a folder
#[derive(Debug, Parser)]
pub struct EnvelopeListCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalArg,

    /// The page number
    #[arg(long, short, value_name = "NUMBER", default_value = "1")]
    pub page: usize,

    /// The page size
    #[arg(long, short = 's', value_name = "NUMBER")]
    pub page_size: Option<usize>,

    #[command(flatten)]
    pub table: MaxTableWidthFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,

    #[command(flatten)]
    pub cache: DisableCacheFlag,
}

impl EnvelopeListCommand {
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
            .unwrap_or(account_config.email_listing_page_size());
        let page = 1.max(self.page) - 1;

        let envelopes = backend.list_envelopes(folder, page_size, page).await?;

        printer.print_table(
            Box::new(envelopes),
            PrintTableOpts {
                format: &account_config.email_reading_format,
                max_width: self.table.max_width,
            },
        )?;

        Ok(())
    }
}
