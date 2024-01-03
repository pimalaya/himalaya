use anyhow::Result;
use clap::Parser;
use dialoguer::Confirm;
#[cfg(feature = "imap")]
use email::folder::delete::imap::DeleteFolderImap;
#[cfg(feature = "maildir")]
use email::folder::delete::maildir::DeleteFolderMaildir;
use log::info;
use std::process;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    folder::arg::name::FolderNameArg,
    printer::Printer,
};

/// Delete a folder.
///
/// All emails from the given folder are definitely deleted. The
/// folder is also deleted after execution of the command.
#[derive(Debug, Parser)]
pub struct FolderDeleteCommand {
    #[command(flatten)]
    pub folder: FolderNameArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderDeleteCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing folder delete command");

        let folder = &self.folder.name;

        let confirm_msg = format!("Do you really want to delete the folder {folder}? All emails will be definitely deleted.");
        let confirm = Confirm::new()
            .with_prompt(confirm_msg)
            .default(false)
            .report(false)
            .interact_opt()?;
        if let Some(false) | None = confirm {
            process::exit(0);
        };

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let delete_folder_kind = toml_account_config.delete_folder_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            delete_folder_kind,
            |builder| match delete_folder_kind {
                Some(BackendKind::Maildir) => {
                    builder.set_delete_folder(|ctx| {
                        ctx.maildir.as_ref().and_then(DeleteFolderMaildir::new)
                    });
                }
                Some(BackendKind::MaildirForSync) => {
                    builder.set_delete_folder(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(DeleteFolderMaildir::new)
                    });
                }
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder
                        .set_delete_folder(|ctx| ctx.imap.as_ref().and_then(DeleteFolderImap::new));
                }
                _ => (),
            },
        )
        .await?;

        backend.delete_folder(&folder).await?;

        printer.print(format!("Folder {folder} successfully deleted!"))
    }
}
