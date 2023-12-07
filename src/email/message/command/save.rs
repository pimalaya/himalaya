use anyhow::Result;
use atty::Stream;
use clap::Parser;
use log::info;
use std::io::{self, BufRead};

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::DisableCacheFlag,
    config::TomlConfig, folder::arg::name::FolderNameArg, printer::Printer,
};

/// Save a message to a folder
#[derive(Debug, Parser)]
pub struct MessageSaveCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    /// The raw message to save
    #[arg(value_name = "MESSAGE", raw = true)]
    pub raw: String,

    #[command(flatten)]
    pub cache: DisableCacheFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageSaveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing message save command");

        let folder = &self.folder.name;
        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;
        let raw_msg = &self.raw;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), true).await?;

        let is_tty = atty::is(Stream::Stdin);
        let is_json = printer.is_json();
        let raw_email = if is_tty || is_json {
            raw_msg.replace("\r", "").replace("\n", "\r\n")
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        backend
            .add_raw_message(folder, raw_email.as_bytes())
            .await?;

        printer.print("Message successfully saved to {folder}!")
    }
}
