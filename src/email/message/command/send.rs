use clap::Parser;
use color_eyre::Result;
use email::backend::feature::BackendFeatureSource;
use std::io::{self, BufRead, IsTerminal};
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig,
    message::arg::MessageRawArg, printer::Printer,
};

/// Send a message.
///
/// This command allows you to send a raw message and to save a copy
/// to your send folder.
#[derive(Debug, Parser)]
pub struct MessageSendCommand {
    #[command(flatten)]
    pub message: MessageRawArg,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageSendCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing send message command");

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let send_message_kind = toml_account_config.send_message_kind().into_iter().chain(
            toml_account_config
                .add_message_kind()
                .filter(|_| account_config.should_save_copy_sent_message()),
        );

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            send_message_kind,
            |builder| {
                builder.set_send_message(BackendFeatureSource::Context);
                builder.set_add_message(BackendFeatureSource::Context);
            },
        )
        .await?;

        let msg = if io::stdin().is_terminal() {
            self.message.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<_>>()
                .join("\r\n")
        };

        backend.send_message_then_save_copy(msg.as_bytes()).await?;

        printer.print("Message successfully sent!")
    }
}
