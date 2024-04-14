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
    flag::arg::ids_and_flags::{into_tuple, IdsAndFlagsArgs},
    folder::arg::name::FolderNameOptionalFlag,
    printer::Printer,
};

/// Remove flag(s) from an envelope.
///
/// This command allows you to remove the given flag(s) from the given
/// envelope(s).
#[derive(Debug, Parser)]
pub struct FlagRemoveCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub args: IdsAndFlagsArgs,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FlagRemoveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing remove flag(s) command");

        let folder = &self.folder.name;
        let (ids, flags) = into_tuple(&self.args.ids_and_flags);
        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let remove_flags_kind = toml_account_config.remove_flags_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            remove_flags_kind,
            |builder| builder.set_remove_flags(BackendFeatureSource::Context),
        )
        .await?;

        backend.remove_flags(folder, &ids, &flags).await?;

        printer.print(format!("Flag(s) {flags} successfully removed!"))
    }
}
