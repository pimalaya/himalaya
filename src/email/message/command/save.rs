use anyhow::Result;
use clap::Parser;
use log::info;
use std::io::{self, BufRead, IsTerminal};

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::CacheDisableFlag,
    config::TomlConfig, folder::arg::name::FolderNameOptionalFlag, message::arg::MessageRawArg,
    printer::Printer,
};

/// Save a message to a folder.
///
/// This command allows you to add a raw message to the given folder.
#[derive(Debug, Parser)]
pub struct MessageSaveCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub message: MessageRawArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageSaveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing message save command");

        let folder = &self.folder.name;
        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), true).await?;

        let is_tty = io::stdin().is_terminal();
        let is_json = printer.is_json();
        let msg = if is_tty || is_json {
            self.message.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        backend.add_raw_message(folder, msg.as_bytes()).await?;

        printer.print(format!("Message successfully saved to {folder}!"))
    }
}
