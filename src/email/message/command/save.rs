use clap::Parser;
use color_eyre::Result;
use email::backend::feature::BackendFeatureSource;
use std::io::{self, BufRead, IsTerminal};
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig,
    folder::arg::name::FolderNameOptionalFlag, message::arg::MessageRawArg, printer::Printer,
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

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageSaveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing save message command");

        let folder = &self.folder.name;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let add_message_kind = toml_account_config.add_message_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            add_message_kind,
            |builder| builder.set_add_message(BackendFeatureSource::Context),
        )
        .await?;

        let is_tty = io::stdin().is_terminal();
        let is_json = printer.is_json();
        let msg = if is_tty || is_json {
            self.message.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        backend.add_message(folder, msg.as_bytes()).await?;

        printer.print(format!("Message successfully saved to {folder}!"))
    }
}
