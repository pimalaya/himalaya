use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::message::copy::imap::CopyMessagesImap;
#[cfg(feature = "maildir")]
use email::message::copy::maildir::CopyMessagesMaildir;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::{SourceFolderNameOptionalFlag, TargetFolderNameArg},
    printer::Printer,
};

/// Copy a message from a source folder to a target folder.
#[derive(Debug, Parser)]
pub struct MessageCopyCommand {
    #[command(flatten)]
    pub source_folder: SourceFolderNameOptionalFlag,

    #[command(flatten)]
    pub target_folder: TargetFolderNameArg,

    #[command(flatten)]
    pub envelopes: EnvelopeIdsArgs,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageCopyCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing copy message(s) command");

        let source = &self.source_folder.name;
        let target = &self.target_folder.name;
        let ids = &self.envelopes.ids;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let copy_messages_kind = toml_account_config.copy_messages_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            copy_messages_kind,
            |builder| match copy_messages_kind {
                Some(BackendKind::Maildir) => {
                    builder.set_copy_messages(|ctx| {
                        ctx.maildir.as_ref().and_then(CopyMessagesMaildir::new)
                    });
                }
                Some(BackendKind::MaildirForSync) => {
                    builder.set_copy_messages(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(CopyMessagesMaildir::new)
                    });
                }
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder
                        .set_copy_messages(|ctx| ctx.imap.as_ref().and_then(CopyMessagesImap::new));
                }
                _ => (),
            },
        )
        .await?;

        backend.copy_messages(source, target, ids).await?;

        printer.print(format!(
            "Message(s) successfully copied from {source} to {target}!"
        ))
    }
}
