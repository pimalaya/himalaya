use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::folder::expunge::imap::ExpungeImapFolder;
#[cfg(feature = "maildir")]
use email::folder::expunge::maildir::ExpungeMaildirFolder;
// #[cfg(feature = "notmuch")]
// use email::folder::expunge::notmuch::ExpungeNotmuchFolder;
use log::info;

#[cfg(any(feature = "imap", feature = "maildir", feature = "account-sync"))]
use crate::backend::BackendKind;
#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig,
    folder::arg::name::FolderNameArg, printer::Printer,
};

/// Expunge a folder.
///
/// The concept of expunging is similar to the IMAP one: it definitely
/// deletes emails from the given folder that contain the "deleted"
/// flag.
#[derive(Debug, Parser)]
pub struct FolderExpungeCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderExpungeCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing expunge folder command");

        let folder = &self.folder.name;
        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let expunge_folder_kind = toml_account_config.expunge_folder_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            expunge_folder_kind,
            |builder| match expunge_folder_kind {
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_expunge_folder(|ctx| {
                        ctx.imap.as_ref().map(ExpungeImapFolder::new_boxed)
                    });
                }
                #[cfg(feature = "maildir")]
                Some(BackendKind::Maildir) => {
                    builder.set_expunge_folder(|ctx| {
                        ctx.maildir.as_ref().map(ExpungeMaildirFolder::new_boxed)
                    });
                }
                #[cfg(feature = "account-sync")]
                Some(BackendKind::MaildirForSync) => {
                    builder.set_expunge_folder(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .map(ExpungeMaildirFolder::new_boxed)
                    });
                }
                #[cfg(feature = "notmuch")]
                Some(BackendKind::Notmuch) => {
                    // TODO
                    // builder.set_expunge_folder(|ctx| {
                    //     ctx.notmuch.as_ref().map(ExpungeNotmuchFolder::new_boxed)
                    // });
                }
                _ => (),
            },
        )
        .await?;

        backend.expunge_folder(folder).await?;

        printer.print(format!("Folder {folder} successfully expunged!"))
    }
}
