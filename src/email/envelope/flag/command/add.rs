use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::flag::add::imap::AddImapFlags;
#[cfg(feature = "maildir")]
use email::flag::add::maildir::AddMaildirFlags;
#[cfg(feature = "notmuch")]
use email::flag::add::notmuch::AddNotmuchFlags;
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

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FlagAddCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing add flag(s) command");

        let folder = &self.folder.name;
        let (ids, flags) = into_tuple(&self.args.ids_and_flags);
        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let add_flags_kind = toml_account_config.add_flags_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            add_flags_kind,
            |builder| match add_flags_kind {
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_add_flags(|ctx| ctx.imap.as_ref().map(AddImapFlags::new_boxed));
                }
                #[cfg(feature = "maildir")]
                Some(BackendKind::Maildir) => {
                    builder
                        .set_add_flags(|ctx| ctx.maildir.as_ref().map(AddMaildirFlags::new_boxed));
                }
                #[cfg(feature = "account-sync")]
                Some(BackendKind::MaildirForSync) => {
                    builder.set_add_flags(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .map(AddMaildirFlags::new_boxed)
                    });
                }
                #[cfg(feature = "notmuch")]
                Some(BackendKind::Notmuch) => {
                    builder
                        .set_add_flags(|ctx| ctx.notmuch.as_ref().map(AddNotmuchFlags::new_boxed));
                }
                _ => (),
            },
        )
        .await?;

        backend.add_flags(folder, &ids, &flags).await?;

        printer.print(format!("Flag(s) {flags} successfully added!"))
    }
}
