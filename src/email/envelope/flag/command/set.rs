use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::flag::set::imap::SetImapFlags;
#[cfg(feature = "maildir")]
use email::flag::set::maildir::SetMaildirFlags;
#[cfg(feature = "notmuch")]
use email::flag::set::notmuch::SetNotmuchFlags;
use log::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    config::TomlConfig,
    flag::arg::ids_and_flags::{into_tuple, IdsAndFlagsArgs},
    folder::arg::name::FolderNameOptionalFlag,
    printer::Printer,
};

/// Replace flag(s) of an envelope.
///
/// This command allows you to replace existing flags of the given
/// envelope(s) with the given flag(s).
#[derive(Debug, Parser)]
pub struct FlagSetCommand {
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

impl FlagSetCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing set flag(s) command");

        let folder = &self.folder.name;
        let (ids, flags) = into_tuple(&self.args.ids_and_flags);
        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let set_flags_kind = toml_account_config.set_flags_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            set_flags_kind,
            |builder| match set_flags_kind {
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_set_flags(|ctx| ctx.imap.as_ref().map(SetImapFlags::new_boxed));
                }
                #[cfg(feature = "maildir")]
                Some(BackendKind::Maildir) => {
                    builder
                        .set_set_flags(|ctx| ctx.maildir.as_ref().map(SetMaildirFlags::new_boxed));
                }
                #[cfg(feature = "account-sync")]
                Some(BackendKind::MaildirForSync) => {
                    builder.set_set_flags(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .map(SetMaildirFlags::new_boxed)
                    });
                }
                #[cfg(feature = "notmuch")]
                Some(BackendKind::Notmuch) => {
                    builder
                        .set_set_flags(|ctx| ctx.notmuch.as_ref().map(SetNotmuchFlags::new_boxed));
                }
                _ => (),
            },
        )
        .await?;

        backend.set_flags(folder, &ids, &flags).await?;

        printer.print(format!("Flag(s) {flags} successfully replaced!"))
    }
}
