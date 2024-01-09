use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::folder::list::imap::ListFoldersImap;
#[cfg(feature = "maildir")]
use email::folder::list::maildir::ListFoldersMaildir;
use log::info;

#[cfg(any(feature = "imap", feature = "maildir", feature = "account-sync"))]
use crate::backend::BackendKind;
#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    config::TomlConfig,
    folder::Folders,
    printer::{PrintTableOpts, Printer},
    ui::arg::max_width::TableMaxWidthFlag,
};

/// List all folders.
///
/// This command allows you to list all exsting folders.
#[derive(Debug, Parser)]
pub struct FolderListCommand {
    #[command(flatten)]
    pub table: TableMaxWidthFlag,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl FolderListCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing list folders command");

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let list_folders_kind = toml_account_config.list_folders_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            list_folders_kind,
            |builder| match list_folders_kind {
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder
                        .set_list_folders(|ctx| ctx.imap.as_ref().and_then(ListFoldersImap::new));
                }
                #[cfg(feature = "maildir")]
                Some(BackendKind::Maildir) => {
                    builder.set_list_folders(|ctx| {
                        ctx.maildir.as_ref().and_then(ListFoldersMaildir::new)
                    });
                }
                #[cfg(feature = "account-sync")]
                Some(BackendKind::MaildirForSync) => {
                    builder.set_list_folders(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(ListFoldersMaildir::new)
                    });
                }
                _ => (),
            },
        )
        .await?;

        let folders: Folders = backend.list_folders().await?.into();

        printer.print_table(
            Box::new(folders),
            PrintTableOpts {
                format: &account_config.get_message_read_format(),
                max_width: self.table.max_width,
            },
        )
    }
}
