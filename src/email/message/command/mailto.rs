use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::message::add::imap::AddMessageImap;
#[cfg(feature = "maildir")]
use email::message::add_with_flags::maildir::AddMessageWithFlagsMaildir;
#[cfg(feature = "sendmail")]
use email::message::send::sendmail::SendMessageSendmail;
#[cfg(feature = "smtp")]
use email::message::send::smtp::SendMessageSmtp;
use log::{debug, info};
use mail_builder::MessageBuilder;
use url::Url;

#[cfg(feature = "sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag,
    backend::{Backend, BackendKind},
    config::TomlConfig,
    printer::Printer,
    ui::editor,
};

/// Parse and edit a message from a mailto URL string.
///
/// This command allows you to edit a message from the mailto format
/// using the editor defined in your environment variable
/// $EDITOR. When the edition process finishes, you can choose between
/// saving or sending the final message.
#[derive(Debug, Parser)]
pub struct MessageMailtoCommand {
    /// The mailto url.
    #[arg()]
    pub url: Url,

    #[cfg(feature = "sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageMailtoCommand {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            url: Url::parse(url)?,
            #[cfg(feature = "sync")]
            cache: Default::default(),
            account: Default::default(),
        })
    }

    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing mailto message command");

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
            |builder| {
                match add_message_kind {
                    #[cfg(feature = "imap")]
                    Some(BackendKind::Imap) => {
                        builder
                            .set_add_message(|ctx| ctx.imap.as_ref().and_then(AddMessageImap::new));
                    }
                    #[cfg(feature = "maildir")]
                    Some(BackendKind::Maildir) => {
                        builder.set_add_message_with_flags(|ctx| {
                            ctx.maildir
                                .as_ref()
                                .and_then(AddMessageWithFlagsMaildir::new)
                        });
                    }
                    #[cfg(feature = "sync")]
                    Some(BackendKind::MaildirForSync) => {
                        builder.set_add_message_with_flags(|ctx| {
                            ctx.maildir_for_sync
                                .as_ref()
                                .and_then(AddMessageWithFlagsMaildir::new)
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

        let mut builder = MessageBuilder::new().to(self.url.path());
        let mut body = String::new();

        for (key, val) in self.url.query_pairs() {
            match key.to_lowercase().as_bytes() {
                b"cc" => builder = builder.cc(val.to_string()),
                b"bcc" => builder = builder.bcc(val.to_string()),
                b"subject" => builder = builder.subject(val),
                b"body" => body += &val,
                _ => (),
            }
        }

        match account_config.find_full_signature() {
            Ok(Some(ref signature)) => builder = builder.text_body(body + "\n\n" + signature),
            Ok(None) => builder = builder.text_body(body),
            Err(err) => {
                debug!("cannot add signature to mailto message, skipping it: {err}");
                debug!("{err:?}");
            }
        }

        let tpl = account_config
            .generate_tpl_interpreter()
            .with_show_only_headers(account_config.get_message_write_headers())
            .build()
            .from_msg_builder(builder)
            .await?;

        editor::edit_tpl_with_editor(&account_config, printer, &backend, tpl).await
    }
}
