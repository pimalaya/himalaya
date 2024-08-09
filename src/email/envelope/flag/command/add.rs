use clap::Parser;
use color_eyre::Result;
use email::backend::feature::BackendFeatureSource;
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    config::TomlConfig,
    flag::arg::ids_and_flags::{into_tuple, IdsAndFlagsArgs},
    folder::arg::name::FolderNameOptionalFlag,
    printer::Printer,
};

/// Add flag(s) to an envelope.
///
/// This command allows you to attach the given flag(s) to the given
/// envelope(s).
#[derive(Debug, Parser)]
pub struct FlagAddCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub args: IdsAndFlagsArgs,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FlagAddCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing add flag(s) command");

        let folder = &self.folder.name;
        let (ids, flags) = into_tuple(&self.args.ids_and_flags);
        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref())?;

        let add_flags_kind = toml_account_config.add_flags_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config,
            add_flags_kind,
            |builder| builder.set_add_flags(BackendFeatureSource::Context),
        )
        .await?;

        backend.add_flags(folder, &ids, &flags).await?;

        printer.out(format!("Flag(s) {flags} successfully added!"))
    }
}
