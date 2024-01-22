use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::message::move_::imap::MoveImapMessages;
#[cfg(feature = "maildir")]
use email::message::move_::maildir::MoveMaildirMessages;
#[cfg(feature = "notmuch")]
use email::message::move_::notmuch::MoveNotmuchMessages;
use log::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::{SourceFolderNameOptionalFlag, TargetFolderNameArg},
    printer::Printer,
};

/// Move a message from a source folder to a target folder.
#[derive(Debug, Parser)]
pub struct MessageMoveCommand {
    #[command(flatten)]
    pub source_folder: SourceFolderNameOptionalFlag,

    #[command(flatten)]
    pub target_folder: TargetFolderNameArg,

    #[command(flatten)]
    pub envelopes: EnvelopeIdsArgs,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageMoveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing move message(s) command");

        let source = &self.source_folder.name;
        let target = &self.target_folder.name;
        let ids = &self.envelopes.ids;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let move_messages_kind = toml_account_config.move_messages_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            move_messages_kind,
            |#[allow(unused)] builder| match move_messages_kind {
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_move_messages(|ctx| {
                        ctx.imap.as_ref().map(MoveImapMessages::new_boxed)
                    });
                }
                #[cfg(feature = "maildir")]
                Some(BackendKind::Maildir) => {
                    builder.set_move_messages(|ctx| {
                        ctx.maildir.as_ref().map(MoveMaildirMessages::new_boxed)
                    });
                }
                #[cfg(feature = "account-sync")]
                Some(BackendKind::MaildirForSync) => {
                    builder.set_move_messages(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .map(MoveMaildirMessages::new_boxed)
                    });
                }
                #[cfg(feature = "notmuch")]
                Some(BackendKind::Notmuch) => {
                    builder.set_move_messages(|ctx| {
                        ctx.notmuch.as_ref().map(MoveNotmuchMessages::new_boxed)
                    });
                }
                _ => (),
            },
        )
        .await?;

        backend.move_messages(source, target, ids).await?;

        printer.print(format!(
            "Message(s) successfully moved from {source} to {target}!"
        ))
    }
}
