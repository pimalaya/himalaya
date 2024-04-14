use clap::Parser;
use color_eyre::{eyre::eyre, Result};
use email::backend::feature::BackendFeatureSource;
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdArg,
    folder::arg::name::FolderNameOptionalFlag,
    message::arg::{body::MessageRawBodyArg, header::HeaderRawArgs, reply::MessageReplyAllArg},
    printer::Printer,
};

/// Generate a template for replying to a message.
///
/// The generated template is prefilled with your email in a From
/// header as well as your signature. The replied message is also
/// prefilled in the body of the template, with all lines prefixed by
/// the symbol greater than ">".
#[derive(Debug, Parser)]
pub struct TemplateReplyCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelope: EnvelopeIdArg,

    #[command(flatten)]
    pub reply: MessageReplyAllArg,

    #[command(flatten)]
    pub headers: HeaderRawArgs,

    #[command(flatten)]
    pub body: MessageRawBodyArg,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateReplyCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing reply template command");

        let folder = &self.folder.name;
        let id = self.envelope.id;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let get_messages_kind = toml_account_config.get_messages_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config.clone(),
            get_messages_kind,
            |builder| builder.set_get_messages(BackendFeatureSource::Context),
        )
        .await?;

        let tpl = backend
            .get_messages(folder, &[id])
            .await?
            .first()
            .ok_or(eyre!("cannot find message {id}"))?
            .to_reply_tpl_builder(account_config)
            .with_headers(self.headers.raw)
            .with_body(self.body.raw())
            .with_reply_all(self.reply.all)
            .build()
            .await?;

        printer.print(tpl)
    }
}
