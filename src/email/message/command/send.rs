use anyhow::Result;
use atty::Stream;
use clap::Parser;
use email::flag::Flag;
use log::info;
use std::io::{self, BufRead};

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::CacheDisableFlag,
    config::TomlConfig, message::arg::body::MessageRawBodyArg, printer::Printer,
};

/// Send a message.
///
/// This command allows you to send a raw message and to save a copy
/// to your send folder.
#[derive(Debug, Parser)]
pub struct MessageSendCommand {
    #[command(flatten)]
    pub body: MessageRawBodyArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageSendCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing message send command");

        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), true).await?;
        let folder = account_config.sent_folder_alias()?;

        let is_tty = atty::is(Stream::Stdin);
        let is_json = printer.is_json();
        let msg = if is_tty || is_json {
            self.body.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        backend.send_raw_message(msg.as_bytes()).await?;

        if account_config.email_sending_save_copy.unwrap_or_default() {
            backend
                .add_raw_message_with_flag(&folder, msg.as_bytes(), Flag::Seen)
                .await?;

            printer.print(format!("Message successfully sent and saved to {folder}!"))
        } else {
            printer.print("Message successfully sent!")
        }
    }
}
