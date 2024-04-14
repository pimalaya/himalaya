use clap::Parser;
use color_eyre::Result;
use email::backend::feature::BackendFeatureSource;
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::{SourceFolderNameOptionalFlag, TargetFolderNameArg},
    printer::Printer,
};

/// Move a message from a source folder to a target folder.
#[derive(Debug, Parser)]
pub struct MessageMoveCommand {
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

impl MessageMoveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing move message(s) command");

        let source = &self.source_folder.name;
        let target = &self.target_folder.name;
        let ids = &self.envelopes.ids;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let move_messages_kind = toml_account_config.move_messages_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            move_messages_kind,
            |builder| builder.set_move_messages(BackendFeatureSource::Context),
        )
        .await?;

        backend.move_messages(source, target, ids).await?;

        printer.print(format!(
            "Message(s) successfully moved from {source} to {target}!"
        ))
    }
}
