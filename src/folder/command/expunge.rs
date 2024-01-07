use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::folder::expunge::imap::ExpungeFolderImap;
#[cfg(feature = "maildir")]
use email::folder::expunge::maildir::ExpungeFolderMaildir;
use log::info;

#[cfg(any(feature = "imap", feature = "maildir", feature = "sync"))]
use crate::backend::BackendKind;
#[cfg(feature = "sync")]
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

    #[cfg(feature = "sync")]
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
            self.account.name.as_ref().map(String::as_str),
            #[cfg(feature = "sync")]
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
                        ctx.imap.as_ref().and_then(ExpungeFolderImap::new)
                    });
                }
                #[cfg(feature = "maildir")]
                Some(BackendKind::Maildir) => {
                    builder.set_expunge_folder(|ctx| {
                        ctx.maildir.as_ref().and_then(ExpungeFolderMaildir::new)
                    });
                }
                #[cfg(feature = "sync")]
                Some(BackendKind::MaildirForSync) => {
                    builder.set_expunge_folder(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(ExpungeFolderMaildir::new)
                    });
                }
                _ => (),
            },
        )
        .await?;

        backend.expunge_folder(&folder).await?;

        printer.print(format!("Folder {folder} successfully expunged!"))
    }
}
