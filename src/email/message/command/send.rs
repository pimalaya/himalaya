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
use log::info;
use std::io::{self, BufRead, IsTerminal};

#[cfg(feature = "sync")]
use crate::cache::arg::disable::CacheDisableFlag;
#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    config::TomlConfig,
    message::arg::MessageRawArg,
    printer::Printer,
};

/// Send a message.
///
/// This command allows you to send a raw message and to save a copy
/// to your send folder.
#[derive(Debug, Parser)]
pub struct MessageSendCommand {
    #[command(flatten)]
    pub message: MessageRawArg,

    #[cfg(feature = "sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageSendCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing send message command");

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_ref().map(String::as_str),
            #[cfg(feature = "sync")]
            self.cache.disable,
        )?;

        let send_message_kind = toml_account_config.send_message_kind();

        #[cfg(feature = "message-add")]
        let add_message_kind = toml_account_config
            .add_message_kind()
            .filter(|_| account_config.should_save_copy_sent_message());
        #[cfg(not(feature = "message-add"))]
        let add_message_kind = None;

        let backend = Backend::new(
            &toml_account_config,
            &account_config,
            send_message_kind.into_iter().chain(add_message_kind),
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

        let msg = if io::stdin().is_terminal() {
            self.message.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<_>>()
                .join("\r\n")
        };

        backend.send_message(msg.as_bytes()).await?;

        printer.print("Message successfully sent!")
    }
}
