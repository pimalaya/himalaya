use clap::Parser;
use color_eyre::Result;
use email::message::Message;
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig,
    email::template::arg::body::TemplateRawBodyArg, message::arg::header::HeaderRawArgs,
    printer::Printer,
};

/// Generate a template for writing a new message from scratch.
///
/// The generated template is prefilled with your email in a From
/// header as well as your signature.
#[derive(Debug, Parser)]
pub struct TemplateWriteCommand {
    #[command(flatten)]
    pub headers: HeaderRawArgs,

    #[command(flatten)]
    pub body: TemplateRawBodyArg,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateWriteCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing write template command");

        let (_, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let tpl = Message::new_tpl_builder(account_config)
            .with_headers(self.headers.raw)
            .with_body(self.body.raw())
            .build()
            .await?;

        printer.print(tpl)
    }
}
