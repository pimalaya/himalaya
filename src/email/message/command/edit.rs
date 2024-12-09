use std::sync::Arc;

use clap::Parser;
use color_eyre::{eyre::eyre, Result};
use email::{backend::feature::BackendFeatureSource, config::Config};
use pimalaya_tui::{
    himalaya::{backend::BackendBuilder, editor},
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, envelope::arg::ids::EnvelopeIdArg,
    folder::arg::name::FolderNameOptionalFlag,
};

/// Edit the message associated to the given envelope id.
///
/// This command allows you to edit the given message using the
/// editor defined in your environment variable $EDITOR. When the
/// edition process finishes, you can choose between saving or sending
/// the final message.
#[derive(Debug, Parser)]
pub struct MessageEditCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelope: EnvelopeIdArg,

    /// List of headers that should be visible at the top of the
    /// message.
    ///
    /// If a given header is not found in the message, it will not be
    /// visible. If no header is given, defaults to the one set up in
    /// your TOML configuration file.
    #[arg(long = "header", short = 'H', value_name = "NAME")]
    pub headers: Vec<String>,

    /// Edit the message on place.
    ///
    /// If set, the original message being edited will be removed at
    /// the end of the command. Useful when you need, for example, to
    /// edit a draft, send it then remove it from the Drafts folder.
    #[arg(long, short = 'p')]
    pub on_place: bool,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageEditCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing edit message command");

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
                    .with_add_message(BackendFeatureSource::Context)
                    .with_send_message(BackendFeatureSource::Context)
                    .with_delete_messages(BackendFeatureSource::Context)
            },
        )
        .build()
        .await?;

        let id = self.envelope.id;
        let tpl = backend
            .get_messages(folder, &[id])
            .await?
            .first()
            .ok_or(eyre!("cannot find message"))?
            .to_read_tpl(&account_config, |mut tpl| {
                if !self.headers.is_empty() {
                    tpl = tpl.with_show_only_headers(&self.headers);
                }

                tpl
            })
            .await?;

        editor::edit_tpl_with_editor(account_config, printer, &backend, tpl).await?;

        if self.on_place {
            backend.delete_messages(folder, &[id]).await?;
        }

        Ok(())
    }
}
