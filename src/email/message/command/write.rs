use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::message::add_raw::imap::AddRawMessageImap;
#[cfg(feature = "maildir")]
use email::message::add_raw_with_flags::maildir::AddRawMessageWithFlagsMaildir;
#[cfg(feature = "sendmail")]
use email::message::send_raw::sendmail::SendRawMessageSendmail;
#[cfg(feature = "smtp")]
use email::message::send_raw::smtp::SendRawMessageSmtp;
use email::message::Message;
use log::info;

use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    cache::arg::disable::CacheDisableFlag,
    config::TomlConfig,
    message::arg::{body::MessageRawBodyArg, header::HeaderRawArgs},
    printer::Printer,
    ui::editor,
};

/// Write a new message.
///
/// This command allows you to write a new message using the editor
/// defined in your environment variable $EDITOR. When the edition
/// process finishes, you can choose between saving or sending the
/// final message.
#[derive(Debug, Parser)]
pub struct MessageWriteCommand {
    #[command(flatten)]
    pub headers: HeaderRawArgs,

    #[command(flatten)]
    pub body: MessageRawBodyArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageWriteCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing write message command");

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            self.cache.disable,
        )?;

        let add_message_kind = toml_account_config.add_raw_message_kind();
        let send_message_kind = toml_account_config.send_raw_message_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            add_message_kind.into_iter().chain(send_message_kind),
            |builder| {
                match add_message_kind {
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
                };

                match send_message_kind {
                    #[cfg(feature = "smtp")]
                    Some(BackendKind::Smtp) => {
                        builder.set_send_raw_message(|ctx| {
                            ctx.smtp.as_ref().and_then(SendRawMessageSmtp::new)
                        });
                    }
                    #[cfg(feature = "sendmail")]
                    Some(BackendKind::Sendmail) => {
                        builder.set_send_raw_message(|ctx| {
                            ctx.sendmail.as_ref().and_then(SendRawMessageSendmail::new)
                        });
                    }
                    _ => (),
                };
            },
        )
        .await?;

        let tpl = Message::new_tpl_builder(&account_config)
            .with_headers(self.headers.raw)
            .with_body(self.body.raw())
            .build()
            .await?;

        editor::edit_tpl_with_editor(&account_config, printer, &backend, tpl).await
    }
}
