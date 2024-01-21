use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::{flag::add::imap::AddImapFlags, message::move_::imap::MoveImapMessages};
#[cfg(feature = "maildir")]
use email::{flag::add::maildir::AddMaildirFlags, message::move_::maildir::MoveMaildirMessages};
use log::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    config::TomlConfig,
    envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::FolderNameOptionalFlag,
    printer::Printer,
};

/// Mark as deleted a message from a folder.
///
/// This command does not really delete the message: if the given
/// folder points to the trash folder, it adds the "deleted" flag to
/// its envelope, otherwise it moves it to the trash folder. Only the
/// expunge folder command truly deletes messages.
#[derive(Debug, Parser)]
pub struct MessageDeleteCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelopes: EnvelopeIdsArgs,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageDeleteCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing delete message(s) command");

        let folder = &self.folder.name;
        let ids = &self.envelopes.ids;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let delete_messages_kind = toml_account_config.delete_messages_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            delete_messages_kind,
            |#[allow(unused)] builder| match delete_messages_kind {
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_move_messages(|ctx| {
                        ctx.imap.as_ref().map(MoveImapMessages::new_boxed)
                    });
                    builder.set_add_flags(|ctx| ctx.imap.as_ref().map(AddImapFlags::new_boxed));
                }
                #[cfg(feature = "maildir")]
                Some(BackendKind::Maildir) => {
                    builder.set_move_messages(|ctx| {
                        ctx.maildir.as_ref().map(MoveMaildirMessages::new_boxed)
                    });
                    builder
                        .set_add_flags(|ctx| ctx.maildir.as_ref().map(AddMaildirFlags::new_boxed));
                }
                #[cfg(feature = "account-sync")]
                Some(BackendKind::MaildirForSync) => {
                    builder.set_move_messages(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .map(MoveMaildirMessages::new_boxed)
                    });
                    builder.set_add_flags(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .map(AddMaildirFlags::new_boxed)
                    });
                }
                _ => (),
            },
        )
        .await?;

        backend.delete_messages(folder, ids).await?;

        printer.print(format!("Message(s) successfully removed from {folder}!"))
    }
}
