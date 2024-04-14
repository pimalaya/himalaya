use clap::Parser;
use color_eyre::Result;
use email::backend::feature::BackendFeatureSource;
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig,
    envelope::arg::ids::EnvelopeIdsArgs, folder::arg::name::FolderNameOptionalFlag,
    printer::Printer,
};

/// Mark as deleted a message from a folder.
///
/// This command does not really delete the message: if the given
/// folder points to the trash folder, it adds the "deleted" flag to
/// its envelope, otherwise it moves it to the trash folder. Only the
/// expunge folder command truly deletes messages.
#[derive(Debug, Parser)]
pub struct MessageDeleteCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelopes: EnvelopeIdsArgs,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageDeleteCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing delete message(s) command");

        let folder = &self.folder.name;
        let ids = &self.envelopes.ids;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let delete_messages_kind = toml_account_config.delete_messages_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            delete_messages_kind,
            |builder| builder.set_delete_messages(BackendFeatureSource::Context),
        )
        .await?;

        backend.delete_messages(folder, ids).await?;

        printer.print(format!("Message(s) successfully removed from {folder}!"))
    }
}
