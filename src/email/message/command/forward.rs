use anyhow::{anyhow, Result};
use atty::Stream;
use clap::Parser;
use log::info;
use std::io::{self, BufRead};

use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    cache::arg::disable::DisableCacheFlag,
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdArg,
    folder::arg::name::FolderNameArg,
    message::arg::{body::BodyRawArg, header::HeaderRawArgs},
    printer::Printer,
    ui::editor,
};

/// Forward a new message
#[derive(Debug, Parser)]
pub struct MessageForwardCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[command(flatten)]
    pub envelope: EnvelopeIdArg,

    /// Forward to all recipients
    #[arg(long, short = 'A')]
    pub all: bool,

    #[command(flatten)]
    pub headers: HeaderRawArgs,

    #[command(flatten)]
    pub body: BodyRawArg,

    #[command(flatten)]
    pub cache: DisableCacheFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageForwardCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing message forward command");

        let folder = &self.folder.name;
        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), true).await?;

        let is_tty = atty::is(Stream::Stdin);
        let is_json = printer.is_json();
        let body = if !self.body.is_empty() && (is_tty || is_json) {
            self.body.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        let id = self.envelope.id;
        let tpl = backend
            .get_messages(folder, &[id])
            .await?
            .first()
            .ok_or(anyhow!("cannot find message"))?
            .to_forward_tpl_builder(&account_config)
            .with_headers(self.headers.raw)
            .with_body(body)
            .build()
            .await?;
        editor::edit_tpl_with_editor(&account_config, printer, &backend, tpl).await
    }
}
