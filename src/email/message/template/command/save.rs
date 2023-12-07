use anyhow::Result;
use atty::Stream;
use clap::Parser;
use log::info;
use mml::MmlCompilerBuilder;
use std::io::{self, BufRead};

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::DisableCacheFlag,
    config::TomlConfig, folder::arg::name::FolderNameArg, printer::Printer,
};

/// Save a template to a folder
#[derive(Debug, Parser)]
pub struct TemplateSaveCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    /// The raw template to save
    #[arg(raw = true, value_delimiter = ' ')]
    pub raw: Vec<String>,

    #[command(flatten)]
    pub cache: DisableCacheFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateSaveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing template save command");

        let folder = &self.folder.name;
        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;

        let is_tty = atty::is(Stream::Stdin);
        let is_json = printer.is_json();
        let tpl = if is_tty || is_json {
            self.raw.join(" ").replace("\r", "")
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<String>>()
                .join("\n")
        };

        #[allow(unused_mut)]
        let mut compiler = MmlCompilerBuilder::new();

        #[cfg(feature = "pgp")]
        compiler.set_some_pgp(config.pgp.clone());

        let msg = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;
        backend.add_raw_message(folder, &msg).await?;

        printer.print(format!("Template successfully saved to {folder}!"))
    }
}
