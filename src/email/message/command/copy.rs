use clap::Parser;
use color_eyre::Result;
use email::backend::feature::BackendFeatureSource;
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::{SourceFolderNameOptionalFlag, TargetFolderNameArg},
    printer::Printer,
};

/// Copy a message from a source folder to a target folder.
#[derive(Debug, Parser)]
pub struct MessageCopyCommand {
    #[command(flatten)]
    pub source_folder: SourceFolderNameOptionalFlag,

    #[command(flatten)]
    pub target_folder: TargetFolderNameArg,

    #[command(flatten)]
    pub envelopes: EnvelopeIdsArgs,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageCopyCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing copy message(s) command");

        let source = &self.source_folder.name;
        let target = &self.target_folder.name;
        let ids = &self.envelopes.ids;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let copy_messages_kind = toml_account_config.copy_messages_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            copy_messages_kind,
            |builder| builder.set_copy_messages(BackendFeatureSource::Context),
        )
        .await?;

        backend.copy_messages(source, target, ids).await?;

        printer.print(format!(
            "Message(s) successfully copied from {source} to {target}!"
        ))
    }
}
