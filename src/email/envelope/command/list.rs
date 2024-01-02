use anyhow::Result;
use clap::Parser;
use email::backend::BackendBuilder;
#[cfg(feature = "imap")]
use email::{envelope::list::imap::ListEnvelopesImap, imap::ImapSessionBuilder};
#[cfg(feature = "maildir")]
use email::{
    envelope::list::maildir::ListEnvelopesMaildir,
    maildir::{config::MaildirConfig, MaildirSessionBuilder},
};
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendContextBuilder, BackendKind},
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    folder::arg::name::FolderNameOptionalArg,
    printer::{PrintTableOpts, Printer},
    ui::arg::max_width::TableMaxWidthFlag,
};

/// List all envelopes.
///
/// This command allows you to list all envelopes included in the
/// given folder.
#[derive(Debug, Parser)]
pub struct ListEnvelopesCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalArg,

    /// The page number.
    ///
    /// The page number starts from 1 (which is the default). Giving a
    /// page number to big will result in a out of bound error.
    #[arg(long, short, value_name = "NUMBER", default_value = "1")]
    pub page: usize,

    /// The page size.
    ///
    /// Determine the amount of envelopes a page should contain.
    #[arg(long, short = 's', value_name = "NUMBER")]
    pub page_size: Option<usize>,

    #[command(flatten)]
    pub table: TableMaxWidthFlag,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl ListEnvelopesCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing envelope list command");

        let folder = &self.folder.name;

        let some_account_name = self.account.name.as_ref().map(String::as_str);
        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(some_account_name, self.cache.disable)?;

        let backend_kind = toml_account_config.list_envelopes_kind();
        let backend_ctx_builder = BackendContextBuilder {
            maildir: toml_account_config
                .maildir
                .as_ref()
                .filter(|_| matches!(backend_kind, Some(BackendKind::Maildir)))
                .map(|mdir_config| {
                    MaildirSessionBuilder::new(account_config.clone(), mdir_config.clone())
                }),
            maildir_for_sync: Some(MaildirConfig {
                root_dir: account_config.get_sync_dir()?,
            })
            .filter(|_| matches!(backend_kind, Some(BackendKind::MaildirForSync)))
            .map(|mdir_config| MaildirSessionBuilder::new(account_config.clone(), mdir_config)),
            #[cfg(feature = "imap")]
            imap: {
                let ctx_builder = toml_account_config
                    .imap
                    .as_ref()
                    .filter(|_| matches!(backend_kind, Some(BackendKind::Imap)))
                    .map(|imap_config| {
                        ImapSessionBuilder::new(account_config.clone(), imap_config.clone())
                            .with_prebuilt_credentials()
                    });
                match ctx_builder {
                    Some(ctx_builder) => Some(ctx_builder.await?),
                    None => None,
                }
            },
            #[cfg(feature = "notmuch")]
            notmuch: toml_account_config
                .notmuch
                .as_ref()
                .filter(|_| matches!(backend_kind, Some(BackendKind::Notmuch)))
                .map(|notmuch_config| {
                    NotmuchSessionBuilder::new(account_config.clone(), notmuch_config.clone())
                }),
            ..Default::default()
        };

        let mut backend_builder = BackendBuilder::new(account_config.clone(), backend_ctx_builder);

        match toml_account_config.list_envelopes_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.maildir.as_ref().and_then(ListEnvelopesMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(ListEnvelopesMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_list_envelopes(|ctx| ctx.imap.as_ref().and_then(ListEnvelopesImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.notmuch.as_ref().and_then(ListEnvelopesNotmuch::new)
                });
            }
            _ => (),
        }

        let backend = Backend::new_v2(toml_account_config.clone(), backend_builder).await?;

        let page_size = self
            .page_size
            .unwrap_or(account_config.get_envelope_list_page_size());
        let page = 1.max(self.page) - 1;

        let envelopes = backend.list_envelopes(folder, page_size, page).await?;

        printer.print_table(
            Box::new(envelopes),
            PrintTableOpts {
                format: &account_config.get_message_read_format(),
                max_width: self.table.max_width,
            },
        )?;

        Ok(())
    }
}
