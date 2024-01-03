use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::envelope::watch::imap::WatchImapEnvelopes;
#[cfg(feature = "maildir")]
use email::envelope::watch::maildir::WatchMaildirEnvelopes;
#[cfg(feature = "notmuch")]
use email::envelope::watch::notmuch::WatchNotmuchEnvelopes;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    folder::arg::name::FolderNameOptionalFlag,
    printer::Printer,
};

/// Watch envelopes for changes.
///
/// This command allows you to watch a folder and execute hooks when
/// changes occur on envelopes.
#[derive(Debug, Parser)]
pub struct WatchEnvelopesCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl WatchEnvelopesCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing watch envelopes command");

        let folder = &self.folder.name;
        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let watch_envelopes_kind = toml_account_config.watch_envelopes_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            watch_envelopes_kind,
            |builder| match watch_envelopes_kind {
                Some(BackendKind::Maildir) => {
                    builder.set_watch_envelopes(|ctx| {
                        ctx.maildir.as_ref().and_then(WatchMaildirEnvelopes::new)
                    });
                }
                Some(BackendKind::MaildirForSync) => {
                    builder.set_watch_envelopes(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(WatchMaildirEnvelopes::new)
                    });
                }
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_watch_envelopes(|ctx| {
                        ctx.imap.as_ref().and_then(WatchImapEnvelopes::new)
                    });
                }
                _ => (),
            },
        )
        .await?;

        printer.print_log(format!(
            "Start watching folder {folder} for envelopes changes…"
        ))?;

        backend.watch_envelopes(&folder).await
    }
}
