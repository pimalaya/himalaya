use std::{
    io::{self, BufRead, IsTerminal},
    sync::Arc,
};

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, config::Config};
use mml::MmlCompilerBuilder;
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, email::template::arg::TemplateRawArg,
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

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateSendCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing send template command");

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref(), |c: &Config, name| {
                c.account(name).ok()
            })?;

        let account_config = Arc::new(account_config);

        let backend = BackendBuilder::new(
            Arc::new(toml_account_config),
            account_config.clone(),
            |builder| {
                builder
                    .without_features()
                    .with_add_message(BackendFeatureSource::Context)
                    .with_send_message(BackendFeatureSource::Context)
            },
        )
        .build()
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

        printer.out("Message successfully sent!")
    }
}
