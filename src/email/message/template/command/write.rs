use anyhow::Result;
use clap::Parser;
use email::message::Message;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    cache::arg::disable::DisableCacheFlag,
    config::TomlConfig,
    message::arg::{body::BodyRawArg, header::HeaderRawArgs},
    printer::Printer,
};

/// Write a new template
#[derive(Debug, Parser)]
pub struct TemplateWriteCommand {
    #[command(flatten)]
    pub headers: HeaderRawArgs,

    #[command(flatten)]
    pub body: BodyRawArg,

    #[command(flatten)]
    pub cache: DisableCacheFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateWriteCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing template write command");

        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (_, account_config) = config.clone().into_account_configs(account, cache)?;

        let tpl: String = Message::new_tpl_builder(&account_config)
            .with_headers(self.headers.raw)
            .with_body(self.body.raw())
            .build()
            .await?
            .into();

        printer.print(tpl)
    }
}
