use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::message::add::imap::AddImapMessage;
#[cfg(feature = "maildir")]
use email::message::add::maildir::AddMaildirMessage;
#[cfg(feature = "sendmail")]
use email::message::send::sendmail::SendMessageSendmail;
#[cfg(feature = "smtp")]
use email::message::send::smtp::SendMessageSmtp;
use email::message::Message;
use log::info;

#[cfg(feature = "sync")]
use crate::cache::arg::disable::CacheDisableFlag;
#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
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

    #[cfg(feature = "sync")]
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
            #[cfg(feature = "sync")]
            self.cache.disable,
        )?;

        let add_message_kind = toml_account_config.add_message_kind();
        let send_message_kind = toml_account_config.send_message_kind();

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            add_message_kind.into_iter().chain(send_message_kind),
            |#[allow(unused)] builder| {
                match add_message_kind {
                    #[cfg(feature = "imap")]
                    Some(BackendKind::Imap) => {
                        builder
                            .set_add_message(|ctx| ctx.imap.as_ref().and_then(AddImapMessage::new));
                    }
                    #[cfg(feature = "maildir")]
                    Some(BackendKind::Maildir) => {
                        builder.set_add_message(|ctx| {
                            ctx.maildir.as_ref().and_then(AddMaildirMessage::new)
                        });
                    }
                    #[cfg(feature = "sync")]
                    Some(BackendKind::MaildirForSync) => {
                        builder.set_add_message(|ctx| {
                            ctx.maildir_for_sync
                                .as_ref()
                                .and_then(AddMaildirMessage::new)
                        });
                    }
                    _ => (),
                };

                match send_message_kind {
                    #[cfg(feature = "smtp")]
                    Some(BackendKind::Smtp) => {
                        builder.set_send_message(|ctx| {
                            ctx.smtp.as_ref().and_then(SendMessageSmtp::new)
                        });
                    }
                    #[cfg(feature = "sendmail")]
                    Some(BackendKind::Sendmail) => {
                        builder.set_send_message(|ctx| {
                            ctx.sendmail.as_ref().and_then(SendMessageSendmail::new)
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
