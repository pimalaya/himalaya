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
    message::arg::{body::MessageRawBodyArg, header::HeaderRawArgs},
    printer::Printer,
    ui::editor,
};

/// Forward a message.
///
/// This command allows you to forward the given message using the
/// editor defined in your environment variable $EDITOR. When the
/// edition process finishes, you can choose between saving or sending
/// the final message.
#[derive(Debug, Parser)]
pub struct MessageForwardCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelope: EnvelopeIdArg,

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

impl MessageForwardCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing forward message command");

        let folder = &self.folder.name;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let add_message_kind = toml_account_config.add_message_kind();
        let send_message_kind = toml_account_config.send_message_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config.clone(),
            add_message_kind.into_iter().chain(send_message_kind),
            |builder| {
                builder.set_add_message(BackendFeatureSource::Context);
                builder.set_send_message(BackendFeatureSource::Context);
            },
        )
        .await?;

        let id = self.envelope.id;
        let tpl = backend
            .get_messages(folder, &[id])
            .await?
            .first()
            .ok_or(eyre!("cannot find message"))?
            .to_forward_tpl_builder(account_config.clone())
            .with_headers(self.headers.raw)
            .with_body(self.body.raw())
            .build()
            .await?;
        editor::edit_tpl_with_editor(account_config, printer, &backend, tpl).await
    }
}
