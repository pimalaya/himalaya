use anyhow::Result;
use atty::Stream;
use clap::Parser;
use email::flag::Flag;
use log::info;
use std::io::{self, BufRead};

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::DisableCacheFlag,
    config::TomlConfig, printer::Printer,
};

/// Send a message from a folder
#[derive(Debug, Parser)]
pub struct MessageSendCommand {
    /// The raw message to send
    #[arg(value_name = "MESSAGE", raw = true)]
    pub raw: String,

    #[command(flatten)]
    pub cache: DisableCacheFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageSendCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing message send command");

        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;
        let raw_msg = &self.raw;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), true).await?;
        let folder = account_config.sent_folder_alias()?;

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

        backend.send_raw_message(raw_email.as_bytes()).await?;

        if account_config.email_sending_save_copy.unwrap_or_default() {
            backend
                .add_raw_message_with_flag(&folder, raw_email.as_bytes(), Flag::Seen)
                .await?;

            printer.print(format!("Message successfully sent and saved to {folder}!"))
        } else {
            printer.print("Message successfully sent!")
        }
    }
}
