use anyhow::Result;
use clap::Parser;
use log::info;
use mml::MmlCompilerBuilder;
use std::io::{self, BufRead, IsTerminal};

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::CacheDisableFlag,
    config::TomlConfig, email::template::arg::TemplateRawArg,
    folder::arg::name::FolderNameOptionalFlag, printer::Printer,
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
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateSaveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing template save command");

        let folder = &self.folder.name;
        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;

        let is_tty = io::stdin().is_terminal();
        let is_json = printer.is_json();
        let tpl = if is_tty || is_json {
            self.template.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<String>>()
                .join("\n")
        };

        #[allow(unused_mut)]
        let mut compiler = MmlCompilerBuilder::new();

        #[cfg(feature = "pgp")]
        compiler.set_some_pgp(account_config.pgp.clone());

        let msg = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;
        backend.add_raw_message(folder, &msg).await?;

        printer.print(format!("Template successfully saved to {folder}!"))
    }
}
