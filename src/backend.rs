use std::{collections::HashMap, ops::Deref};

use anyhow::{anyhow, Result};
use async_trait::async_trait;

#[cfg(feature = "imap-backend")]
use email::imap::{ImapSessionBuilder, ImapSessionSync};
#[cfg(feature = "smtp-sender")]
use email::smtp::{SmtpClientBuilder, SmtpClientSync};
use email::{
    account::AccountConfig,
    config::Config,
    email::{
        envelope::list::{imap::ListEnvelopesImap, maildir::ListEnvelopesMaildir},
        message::send_raw::{sendmail::SendRawMessageSendmail, smtp::SendRawMessageSmtp},
    },
    maildir::{MaildirSessionBuilder, MaildirSessionSync},
    sendmail::SendmailContext,
};
use serde::{Deserialize, Serialize};

use crate::config::DeserializedConfig;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    Maildir,
    #[cfg(feature = "imap-backend")]
    Imap,
    #[cfg(feature = "notmuch-backend")]
    Notmuch,
    #[cfg(feature = "smtp-sender")]
    Smtp,
    Sendmail,
}

#[derive(Clone, Default)]
pub struct BackendContextBuilder {
    #[cfg(feature = "imap-backend")]
    imap: Option<ImapSessionBuilder>,
    maildir: Option<MaildirSessionBuilder>,
    #[cfg(feature = "smtp-sender")]
    smtp: Option<SmtpClientBuilder>,
    sendmail: Option<SendmailContext>,
}

#[async_trait]
impl email::backend::BackendContextBuilder for BackendContextBuilder {
    type Context = BackendContext;

    async fn build(self) -> Result<Self::Context> {
        let mut ctx = BackendContext::default();

        if let Some(maildir) = self.maildir {
            ctx.maildir = Some(maildir.build().await?);
        }

        #[cfg(feature = "imap-backend")]
        if let Some(imap) = self.imap {
            ctx.imap = Some(imap.build().await?);
        }

        #[cfg(feature = "notmuch-backend")]
        if let Some(notmuch) = self.notmuch {
            ctx.notmuch = Some(notmuch.build().await?);
        }

        #[cfg(feature = "smtp-sender")]
        if let Some(smtp) = self.smtp {
            ctx.smtp = Some(smtp.build().await?);
        }

        if let Some(sendmail) = self.sendmail {
            ctx.sendmail = Some(sendmail.build().await?);
        }

        Ok(ctx)
    }
}

#[derive(Default)]
pub struct BackendContext {
    #[cfg(feature = "imap-backend")]
    pub imap: Option<ImapSessionSync>,
    pub maildir: Option<MaildirSessionSync>,
    #[cfg(feature = "smtp-sender")]
    pub smtp: Option<SmtpClientSync>,
    pub sendmail: Option<SendmailContext>,
}

pub struct BackendBuilder(pub email::backend::BackendBuilder<BackendContextBuilder>);

impl BackendBuilder {
    pub async fn new(config: DeserializedConfig, account_name: Option<&str>) -> Result<Self> {
        let (account_name, mut deserialized_account_config) = match account_name {
            Some("default") | Some("") | None => config
                .accounts
                .iter()
                .find_map(|(name, account)| {
                    account
                        .default
                        .filter(|default| *default == true)
                        .map(|_| (name.to_owned(), account.clone()))
                })
                .ok_or_else(|| anyhow!("cannot find default account")),
            Some(name) => config
                .accounts
                .get(name)
                .map(|account| (name.to_owned(), account.clone()))
                .ok_or_else(|| anyhow!("cannot find account {name}")),
        }?;

        println!(
            "deserialized_account_config: {:#?}",
            deserialized_account_config
        );

        #[cfg(feature = "imap-backend")]
        if let Some(imap_config) = deserialized_account_config.imap.as_mut() {
            imap_config
                .auth
                .replace_undefined_keyring_entries(&account_name);
        }

        #[cfg(feature = "smtp-sender")]
        if let Some(smtp_config) = deserialized_account_config.smtp.as_mut() {
            smtp_config
                .auth
                .replace_undefined_keyring_entries(&account_name);
        }

        let config = Config {
            display_name: config.display_name,
            signature_delim: config.signature_delim,
            signature: config.signature,
            downloads_dir: config.downloads_dir,

            folder_listing_page_size: config.folder_listing_page_size,
            folder_aliases: config.folder_aliases,

            email_listing_page_size: config.email_listing_page_size,
            email_listing_datetime_fmt: config.email_listing_datetime_fmt,
            email_listing_datetime_local_tz: config.email_listing_datetime_local_tz,
            email_reading_headers: config.email_reading_headers,
            email_reading_format: config.email_reading_format,
            email_writing_headers: config.email_writing_headers,
            email_sending_save_copy: config.email_sending_save_copy,
            email_hooks: config.email_hooks,

            accounts: HashMap::from_iter(config.accounts.clone().into_iter().map(
                |(name, config)| {
                    (
                        name.clone(),
                        AccountConfig {
                            name,
                            email: config.email,
                            display_name: config.display_name,
                            signature_delim: config.signature_delim,
                            signature: config.signature,
                            downloads_dir: config.downloads_dir,

                            folder_listing_page_size: config.folder_listing_page_size,
                            folder_aliases: config.folder_aliases.unwrap_or_default(),

                            email_listing_page_size: config.email_listing_page_size,
                            email_listing_datetime_fmt: config.email_listing_datetime_fmt,
                            email_listing_datetime_local_tz: config.email_listing_datetime_local_tz,

                            email_reading_headers: config.email_reading_headers,
                            email_reading_format: config.email_reading_format.unwrap_or_default(),
                            email_writing_headers: config.email_writing_headers,
                            email_sending_save_copy: config.email_sending_save_copy,
                            email_hooks: config.email_hooks.unwrap_or_default(),

                            sync: config.sync,
                            sync_dir: config.sync_dir,
                            sync_folders_strategy: config.sync_folders_strategy.unwrap_or_default(),

                            #[cfg(feature = "pgp")]
                            pgp: config.pgp,
                        },
                    )
                },
            )),
        };

        let account_config = config.account(&account_name)?;

        let backend_ctx_builder = BackendContextBuilder {
            maildir: deserialized_account_config
                .maildir
                .as_ref()
                .map(|mdir_config| {
                    MaildirSessionBuilder::new(account_config.clone(), mdir_config.clone())
                }),
            #[cfg(feature = "imap-backend")]
            imap: deserialized_account_config
                .imap
                .as_ref()
                .map(|imap_config| {
                    ImapSessionBuilder::new(account_config.clone(), imap_config.clone())
                }),
            #[cfg(feature = "smtp-sender")]
            smtp: deserialized_account_config
                .smtp
                .as_ref()
                .map(|smtp_config| {
                    SmtpClientBuilder::new(account_config.clone(), smtp_config.clone())
                }),
            sendmail: deserialized_account_config
                .sendmail
                .as_ref()
                .map(|sendmail_config| {
                    SendmailContext::new(account_config.clone(), sendmail_config.clone())
                }),
            ..Default::default()
        };

        let mut backend_builder =
            email::backend::BackendBuilder::new(account_config.clone(), backend_ctx_builder);

        let list_envelopes = deserialized_account_config
            .envelope
            .as_ref()
            .and_then(|envelope| envelope.list.as_ref())
            .and_then(|send| send.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match list_envelopes {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.maildir.as_ref().and_then(ListEnvelopesMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_list_envelopes(|ctx| ctx.imap.as_ref().and_then(ListEnvelopesImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.notmuch.as_ref().and_then(ListEnvelopesNotmuch::new)
                });
            }
            _ => (),
        }

        let send_msg = deserialized_account_config
            .message
            .as_ref()
            .and_then(|msg| msg.send.as_ref())
            .and_then(|send| send.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match send_msg {
            #[cfg(feature = "smtp-sender")]
            Some(BackendKind::Smtp) => {
                backend_builder = backend_builder.with_send_raw_message(|ctx| {
                    ctx.smtp.as_ref().and_then(SendRawMessageSmtp::new)
                });
            }
            Some(BackendKind::Sendmail) => {
                backend_builder = backend_builder.with_send_raw_message(|ctx| {
                    ctx.sendmail.as_ref().and_then(SendRawMessageSendmail::new)
                });
            }
            _ => (),
        }

        Ok(Self(backend_builder))
    }
}

impl Deref for BackendBuilder {
    type Target = email::backend::BackendBuilder<BackendContextBuilder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type Backend = email::backend::Backend<BackendContext>;
