use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::folder::add::imap::AddFolderImap;
#[cfg(feature = "maildir")]
use email::folder::add::maildir::AddFolderMaildir;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    folder::arg::name::FolderNameArg,
    printer::Printer,
};

/// Create a new folder.
///
/// This command allows you to create a new folder using the given
/// name.
#[derive(Debug, Parser)]
pub struct FolderCreateCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderCreateCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing create folder command");

        let folder = &self.folder.name;
        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let add_folder_kind = toml_account_config.add_folder_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            add_folder_kind,
            |builder| match add_folder_kind {
                Some(BackendKind::Maildir) => {
                    builder
                        .set_add_folder(|ctx| ctx.maildir.as_ref().and_then(AddFolderMaildir::new));
                }
                Some(BackendKind::MaildirForSync) => {
                    builder.set_add_folder(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(AddFolderMaildir::new)
                    });
                }
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_add_folder(|ctx| ctx.imap.as_ref().and_then(AddFolderImap::new));
                }
                _ => (),
            },
        )
        .await?;

        backend.add_folder(&folder).await?;

        printer.print(format!("Folder {folder} successfully created!"))
    }
}
