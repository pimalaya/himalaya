use std::sync::Arc;

use clap::Parser;
use color_eyre::{eyre::eyre, Result};
use email::{backend::feature::BackendFeatureSource, config::Config};
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag,
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdArg,
    folder::arg::name::FolderNameOptionalFlag,
    message::arg::{body::MessageRawBodyArg, header::HeaderRawArgs},
};

/// Generate a template for forwarding a message.
///
/// The generated template is prefilled with your email in a From
/// header as well as your signature. The forwarded message is also
/// prefilled in the body of the template, prefixed by a separator.
#[derive(Debug, Parser)]
pub struct TemplateForwardCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelope: EnvelopeIdArg,

    #[command(flatten)]
    pub headers: HeaderRawArgs,

    #[command(flatten)]
    pub body: MessageRawBodyArg,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateForwardCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing forward template command");

        let folder = &self.folder.name;

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
                    .with_get_messages(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        let id = self.envelope.id;
        let tpl = backend
            .get_messages(folder, &[id])
            .await?
            .first()
            .ok_or(eyre!("cannot find message {id}"))?
            .to_forward_tpl_builder(account_config)
            .with_headers(self.headers.raw)
            .with_body(self.body.raw())
            .build()
            .await?;

        printer.out(tpl)
    }
}
