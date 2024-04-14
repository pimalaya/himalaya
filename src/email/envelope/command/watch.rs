use clap::Parser;
use color_eyre::Result;
use email::backend::feature::BackendFeatureSource;
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig,
    folder::arg::name::FolderNameOptionalFlag, printer::Printer,
};

/// Watch envelopes for changes.
///
/// This command allows you to watch a folder and execute hooks when
/// changes occur on envelopes.
#[derive(Debug, Parser)]
pub struct WatchEnvelopesCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl WatchEnvelopesCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing watch envelopes command");

        let folder = &self.folder.name;
        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let watch_envelopes_kind = toml_account_config.watch_envelopes_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            watch_envelopes_kind,
            |builder| builder.set_watch_envelopes(BackendFeatureSource::Context),
        )
        .await?;

        printer.print_log(format!(
            "Start watching folder {folder} for envelopes changes…"
        ))?;

        backend.watch_envelopes(folder).await
    }
}
