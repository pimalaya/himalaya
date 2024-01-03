use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::flag::remove::imap::RemoveFlagsImap;
#[cfg(feature = "maildir")]
use email::flag::remove::maildir::RemoveFlagsMaildir;
#[cfg(feature = "notmuch")]
use email::flag::remove::notmuch::RemoveFlagsNotmuch;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
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
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let remove_flags_kind = toml_account_config.remove_flags_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            remove_flags_kind,
            |builder| match remove_flags_kind {
                Some(BackendKind::Maildir) => {
                    builder.set_remove_flags(|ctx| {
                        ctx.maildir.as_ref().and_then(RemoveFlagsMaildir::new)
                    });
                }
                Some(BackendKind::MaildirForSync) => {
                    builder.set_remove_flags(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(RemoveFlagsMaildir::new)
                    });
                }
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder
                        .set_remove_flags(|ctx| ctx.imap.as_ref().and_then(RemoveFlagsImap::new));
                }
                _ => (),
            },
        )
        .await?;

        backend.remove_flags(folder, &ids, &flags).await?;

        printer.print(format!("Flag(s) {flags} successfully removed!"))
    }
}
