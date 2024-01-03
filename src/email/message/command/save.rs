use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::message::add_raw::imap::AddRawMessageImap;
#[cfg(feature = "maildir")]
use email::message::add_raw_with_flags::maildir::AddRawMessageWithFlagsMaildir;
use log::info;
use std::io::{self, BufRead, IsTerminal};

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
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
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let add_message_kind = toml_account_config.add_raw_message_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            add_message_kind,
            |builder| match add_message_kind {
                Some(BackendKind::Maildir) => {
                    builder.set_add_raw_message_with_flags(|ctx| {
                        ctx.maildir
                            .as_ref()
                            .and_then(AddRawMessageWithFlagsMaildir::new)
                    });
                }
                Some(BackendKind::MaildirForSync) => {
                    builder.set_add_raw_message_with_flags(|ctx| {
                        ctx.maildir_for_sync
                            .as_ref()
                            .and_then(AddRawMessageWithFlagsMaildir::new)
                    });
                }
                #[cfg(feature = "imap")]
                Some(BackendKind::Imap) => {
                    builder.set_add_raw_message(|ctx| {
                        ctx.imap.as_ref().and_then(AddRawMessageImap::new)
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

        backend.add_raw_message(folder, msg.as_bytes()).await?;

        printer.print(format!("Message successfully saved to {folder}!"))
    }
}
