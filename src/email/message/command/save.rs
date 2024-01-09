use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::message::add::imap::AddImapMessage;
#[cfg(feature = "maildir")]
use email::message::add::maildir::AddMaildirMessage;
use log::info;
use std::io::{self, BufRead, IsTerminal};

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    config::TomlConfig,
    folder::arg::name::FolderNameOptionalFlag,
    message::arg::MessageRawArg,
    printer::Printer,
};

/// Save a message to a folder.
///
/// This command allows you to add a raw message to the given folder.
#[derive(Debug, Parser)]
pub struct MessageSaveCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub message: MessageRawArg,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageSaveCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing save message command");

        let folder = &self.folder.name;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let add_message_kind = toml_account_config.add_message_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            add_message_kind,
            |#[allow(unused)] builder| match add_message_kind {
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_add_message(|ctx| ctx.imap.as_ref().and_then(AddImapMessage::new));
                }
                #[cfg(feature = "maildir")]
                Some(BackendKind::Maildir) => {
                    builder.set_add_message(|ctx| {
                        ctx.maildir.as_ref().and_then(AddMaildirMessage::new)
                    });
                }
                #[cfg(feature = "account-sync")]
                Some(BackendKind::MaildirForSync) => {
                    builder.set_add_message(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(AddMaildirMessage::new)
                    });
                }
                _ => (),
            },
        )
        .await?;

        let is_tty = io::stdin().is_terminal();
        let is_json = printer.is_json();
        let msg = if is_tty || is_json {
            self.message.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        backend.add_message(folder, msg.as_bytes()).await?;

        printer.print(format!("Message successfully saved to {folder}!"))
    }
}
