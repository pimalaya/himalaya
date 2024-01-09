use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::folder::add::imap::AddFolderImap;
#[cfg(feature = "maildir")]
use email::folder::add::maildir::AddFolderMaildir;
use log::info;

#[cfg(any(feature = "imap", feature = "maildir", feature = "account-sync"))]
use crate::backend::BackendKind;
#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, config::TomlConfig,
    folder::arg::name::FolderNameArg, printer::Printer,
};

/// Create a new folder.
///
/// This command allows you to create a new folder using the given
/// name.
#[derive(Debug, Parser)]
pub struct AddFolderCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl AddFolderCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing create folder command");

        let folder = &self.folder.name;
        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let add_folder_kind = toml_account_config.add_folder_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            add_folder_kind,
            |builder| match add_folder_kind {
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_add_folder(|ctx| ctx.imap.as_ref().and_then(AddFolderImap::new));
                }
                #[cfg(feature = "maildir")]
                Some(BackendKind::Maildir) => {
                    builder
                        .set_add_folder(|ctx| ctx.maildir.as_ref().and_then(AddFolderMaildir::new));
                }
                #[cfg(feature = "account-sync")]
                Some(BackendKind::MaildirForSync) => {
                    builder.set_add_folder(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(AddFolderMaildir::new)
                    });
                }
                _ => (),
            },
        )
        .await?;

        backend.add_folder(&folder).await?;

        printer.print(format!("Folder {folder} successfully created!"))
    }
}
