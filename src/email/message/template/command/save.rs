use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, config::Config};
use mml::MmlCompilerBuilder;
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use std::{
    io::{self, BufRead, IsTerminal},
    sync::Arc,
};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, email::template::arg::TemplateRawArg,
    folder::arg::name::FolderNameOptionalFlag,
};

/// Save a template to a folder.
///
/// This command allows you to save a template to the given
/// folder. The template is compiled into a MIME message before being
/// saved to the folder. If you want to save a raw message, use the
/// message save command instead.
#[derive(Debug, Parser)]
pub struct TemplateSaveCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub template: TemplateRawArg,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateSaveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing save template command");

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
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        let is_tty = io::stdin().is_terminal();
        let is_json = printer.is_json();
        let tpl = if is_tty || is_json {
            self.template.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
                .join("\n")
        };

        #[allow(unused_mut)]
        let mut compiler = MmlCompilerBuilder::new();

        #[cfg(feature = "pgp")]
        compiler.set_some_pgp(account_config.pgp.clone());

        let msg = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

        backend.add_message(folder, &msg).await?;

        printer.out(format!("Template successfully saved to {folder}!\n"))
    }
}
