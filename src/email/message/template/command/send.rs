use clap::Parser;
use color_eyre::Result;
use email::backend::feature::BackendFeatureSource;
use mml::MmlCompilerBuilder;
use std::io::{self, BufRead, IsTerminal};
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig,
    email::template::arg::TemplateRawArg, printer::Printer,
};

/// Send a template.
///
/// This command allows you to send a template and save a copy to the
/// sent folder. The template is compiled into a MIME message before
/// being sent. If you want to send a raw message, use the message
/// send command instead.
#[derive(Debug, Parser)]
pub struct TemplateSendCommand {
    #[command(flatten)]
    pub template: TemplateRawArg,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateSendCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing send template command");

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
            account_config.clone(),
            send_message_kind,
            |builder| {
                builder.set_send_message(BackendFeatureSource::Context);
                builder.set_add_message(BackendFeatureSource::Context);
            },
        )
        .await?;

        let tpl = if io::stdin().is_terminal() {
            self.template.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<_>>()
                .join("\n")
        };

        #[allow(unused_mut)]
        let mut compiler = MmlCompilerBuilder::new();

        #[cfg(feature = "pgp")]
        compiler.set_some_pgp(account_config.pgp.clone());

        let msg = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

        backend.send_message_then_save_copy(&msg).await?;

        printer.print("Message successfully sent!")
    }
}
