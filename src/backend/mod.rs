pub mod config;
pub(crate) mod wizard;

use anyhow::Result;
use async_trait::async_trait;
use std::ops::Deref;

#[cfg(feature = "imap")]
use email::imap::{ImapSessionBuilder, ImapSessionSync};
#[cfg(feature = "smtp")]
use email::smtp::{SmtpClientBuilder, SmtpClientSync};
use email::{
    account::config::AccountConfig,
    envelope::{
        get::{imap::GetEnvelopeImap, maildir::GetEnvelopeMaildir},
        list::{imap::ListEnvelopesImap, maildir::ListEnvelopesMaildir},
        watch::{imap::WatchImapEnvelopes, maildir::WatchMaildirEnvelopes},
        Id, SingleId,
    },
    flag::{
        add::{imap::AddFlagsImap, maildir::AddFlagsMaildir},
        remove::{imap::RemoveFlagsImap, maildir::RemoveFlagsMaildir},
        set::{imap::SetFlagsImap, maildir::SetFlagsMaildir},
        Flag, Flags,
    },
    folder::{
        add::{imap::AddFolderImap, maildir::AddFolderMaildir},
        delete::{imap::DeleteFolderImap, maildir::DeleteFolderMaildir},
        expunge::{imap::ExpungeFolderImap, maildir::ExpungeFolderMaildir},
        list::{imap::ListFoldersImap, maildir::ListFoldersMaildir},
        purge::imap::PurgeFolderImap,
    },
    maildir::{config::MaildirConfig, MaildirSessionBuilder, MaildirSessionSync},
    message::{
        add_raw::imap::AddRawMessageImap,
        add_raw_with_flags::{
            imap::AddRawMessageWithFlagsImap, maildir::AddRawMessageWithFlagsMaildir,
        },
        copy::{imap::CopyMessagesImap, maildir::CopyMessagesMaildir},
        get::imap::GetMessagesImap,
        move_::{imap::MoveMessagesImap, maildir::MoveMessagesMaildir},
        peek::{imap::PeekMessagesImap, maildir::PeekMessagesMaildir},
        send_raw::{sendmail::SendRawMessageSendmail, smtp::SendRawMessageSmtp},
        Messages,
    },
    sendmail::SendmailContext,
};
use serde::{Deserialize, Serialize};

use crate::{account::config::TomlAccountConfig, cache::IdMapper, envelope::Envelopes};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    Maildir,
    #[serde(skip_deserializing)]
    MaildirForSync,
    #[cfg(feature = "imap")]
    Imap,
    #[cfg(feature = "notmuch")]
    Notmuch,
    #[cfg(feature = "smtp")]
    Smtp,
    Sendmail,
}

impl ToString for BackendKind {
    fn to_string(&self) -> String {
        let kind = match self {
            Self::Maildir => "Maildir",
            Self::MaildirForSync => "Maildir",
            #[cfg(feature = "imap")]
            Self::Imap => "IMAP",
            #[cfg(feature = "notmuch")]
            Self::Notmuch => "Notmuch",
            #[cfg(feature = "smtp")]
            Self::Smtp => "SMTP",
            Self::Sendmail => "Sendmail",
        };

        kind.to_string()
    }
}

#[derive(Clone, Default)]
pub struct BackendContextBuilder {
    maildir: Option<MaildirSessionBuilder>,
    maildir_for_sync: Option<MaildirSessionBuilder>,
    #[cfg(feature = "imap")]
    imap: Option<ImapSessionBuilder>,
    #[cfg(feature = "smtp")]
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

        if let Some(maildir) = self.maildir_for_sync {
            ctx.maildir_for_sync = Some(maildir.build().await?);
        }

        #[cfg(feature = "imap")]
        if let Some(imap) = self.imap {
            ctx.imap = Some(imap.build().await?);
        }

        #[cfg(feature = "notmuch")]
        if let Some(notmuch) = self.notmuch {
            ctx.notmuch = Some(notmuch.build().await?);
        }

        #[cfg(feature = "smtp")]
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
    pub maildir: Option<MaildirSessionSync>,
    pub maildir_for_sync: Option<MaildirSessionSync>,
    #[cfg(feature = "imap")]
    pub imap: Option<ImapSessionSync>,
    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpClientSync>,
    pub sendmail: Option<SendmailContext>,
}

pub struct BackendBuilder {
    toml_account_config: TomlAccountConfig,
    builder: email::backend::BackendBuilder<BackendContextBuilder>,
}

impl BackendBuilder {
    pub async fn new(
        toml_account_config: TomlAccountConfig,
        account_config: AccountConfig,
        with_sending: bool,
    ) -> Result<Self> {
        let used_backends = toml_account_config.get_used_backends();

        let is_maildir_used = used_backends.contains(&BackendKind::Maildir);
        let is_maildir_for_sync_used = used_backends.contains(&BackendKind::MaildirForSync);
        #[cfg(feature = "imap")]
        let is_imap_used = used_backends.contains(&BackendKind::Imap);
        #[cfg(feature = "notmuch")]
        let is_notmuch_used = used_backends.contains(&BackendKind::Notmuch);
        #[cfg(feature = "smtp")]
        let is_smtp_used = used_backends.contains(&BackendKind::Smtp);
        let is_sendmail_used = used_backends.contains(&BackendKind::Sendmail);

        let backend_ctx_builder = BackendContextBuilder {
            maildir: toml_account_config
                .maildir
                .as_ref()
                .filter(|_| is_maildir_used)
                .map(|mdir_config| {
                    MaildirSessionBuilder::new(account_config.clone(), mdir_config.clone())
                }),
            maildir_for_sync: Some(MaildirConfig {
                root_dir: account_config.get_sync_dir()?,
            })
            .filter(|_| is_maildir_for_sync_used)
            .map(|mdir_config| MaildirSessionBuilder::new(account_config.clone(), mdir_config)),

            #[cfg(feature = "imap")]
            imap: {
                let ctx_builder = toml_account_config
                    .imap
                    .as_ref()
                    .filter(|_| is_imap_used)
                    .map(|imap_config| {
                        ImapSessionBuilder::new(account_config.clone(), imap_config.clone())
                            .with_prebuilt_credentials()
                    });

                match ctx_builder {
                    Some(ctx_builder) => Some(ctx_builder.await?),
                    None => None,
                }
            },
            #[cfg(feature = "notmuch")]
            notmuch: toml_account_config
                .notmuch
                .as_ref()
                .filter(|_| is_notmuch_used)
                .map(|notmuch_config| {
                    NotmuchSessionBuilder::new(account_config.clone(), notmuch_config.clone())
                }),
            #[cfg(feature = "smtp")]
            smtp: toml_account_config
                .smtp
                .as_ref()
                .filter(|_| with_sending)
                .filter(|_| is_smtp_used)
                .map(|smtp_config| {
                    SmtpClientBuilder::new(account_config.clone(), smtp_config.clone())
                }),
            sendmail: toml_account_config
                .sendmail
                .as_ref()
                .filter(|_| with_sending)
                .filter(|_| is_sendmail_used)
                .map(|sendmail_config| {
                    SendmailContext::new(account_config.clone(), sendmail_config.clone())
                }),
        };

        let mut backend_builder =
            email::backend::BackendBuilder::new(account_config.clone(), backend_ctx_builder);

        match toml_account_config.add_folder_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.maildir.as_ref().and_then(AddFolderMaildir::new));
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_add_folder(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(AddFolderMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.imap.as_ref().and_then(AddFolderImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.notmuch.as_ref().and_then(AddFolderNotmuch::new));
            }
            _ => (),
        }

        match toml_account_config.list_folders_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.maildir.as_ref().and_then(ListFoldersMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(ListFoldersMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_list_folders(|ctx| ctx.imap.as_ref().and_then(ListFoldersImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.notmuch.as_ref().and_then(ListFoldersNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.expunge_folder_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_expunge_folder(|ctx| {
                    ctx.maildir.as_ref().and_then(ExpungeFolderMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_expunge_folder(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(ExpungeFolderMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_expunge_folder(|ctx| ctx.imap.as_ref().and_then(ExpungeFolderImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_expunge_folder(|ctx| {
                    ctx.notmuch.as_ref().and_then(ExpungeFolderNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.purge_folder_kind() {
            // TODO
            // Some(BackendKind::Maildir) => {
            //     backend_builder = backend_builder
            //         .with_purge_folder(|ctx| ctx.maildir.as_ref().and_then(PurgeFolderMaildir::new));
            // }
            // TODO
            // Some(BackendKind::MaildirForSync) => {
            //     backend_builder = backend_builder
            //         .with_purge_folder(|ctx| ctx.maildir_for_sync.as_ref().and_then(PurgeFolderMaildir::new));
            // }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_purge_folder(|ctx| ctx.imap.as_ref().and_then(PurgeFolderImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_purge_folder(|ctx| {
                    ctx.notmuch.as_ref().and_then(PurgeFolderNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.delete_folder_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_delete_folder(|ctx| {
                    ctx.maildir.as_ref().and_then(DeleteFolderMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_delete_folder(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(DeleteFolderMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_delete_folder(|ctx| ctx.imap.as_ref().and_then(DeleteFolderImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_delete_folder(|ctx| {
                    ctx.notmuch.as_ref().and_then(DeleteFolderNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.backend {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_watch_envelopes(|ctx| {
                    ctx.maildir.as_ref().and_then(WatchMaildirEnvelopes::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_watch_envelopes(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(WatchMaildirEnvelopes::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder.with_watch_envelopes(|ctx| {
                    ctx.imap.as_ref().and_then(WatchImapEnvelopes::new)
                });
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_watch_envelopes(|ctx| {
                    ctx.notmuch.as_ref().and_then(WatchNotmuchEnvelopes::new)
                });
            }
            _ => (),
        }

        match toml_account_config.get_envelope_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.maildir.as_ref().and_then(GetEnvelopeMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(GetEnvelopeMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_get_envelope(|ctx| ctx.imap.as_ref().and_then(GetEnvelopeImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.notmuch.as_ref().and_then(GetEnvelopeNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.list_envelopes_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.maildir.as_ref().and_then(ListEnvelopesMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(ListEnvelopesMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_list_envelopes(|ctx| ctx.imap.as_ref().and_then(ListEnvelopesImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.notmuch.as_ref().and_then(ListEnvelopesNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.add_flags_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.maildir.as_ref().and_then(AddFlagsMaildir::new));
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_add_flags(|ctx| {
                    ctx.maildir_for_sync.as_ref().and_then(AddFlagsMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.imap.as_ref().and_then(AddFlagsImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.notmuch.as_ref().and_then(AddFlagsNotmuch::new));
            }
            _ => (),
        }

        match toml_account_config.set_flags_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.maildir.as_ref().and_then(SetFlagsMaildir::new));
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_set_flags(|ctx| {
                    ctx.maildir_for_sync.as_ref().and_then(SetFlagsMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.imap.as_ref().and_then(SetFlagsImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.notmuch.as_ref().and_then(SetFlagsNotmuch::new));
            }
            _ => (),
        }

        match toml_account_config.remove_flags_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.maildir.as_ref().and_then(RemoveFlagsMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(RemoveFlagsMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_remove_flags(|ctx| ctx.imap.as_ref().and_then(RemoveFlagsImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.notmuch.as_ref().and_then(RemoveFlagsNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.send_raw_message_kind() {
            #[cfg(feature = "smtp")]
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

        match toml_account_config.add_raw_message_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_add_raw_message_with_flags(|ctx| {
                    ctx.maildir
                        .as_ref()
                        .and_then(AddRawMessageWithFlagsMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_add_raw_message_with_flags(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(AddRawMessageWithFlagsMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_raw_message(|ctx| ctx.imap.as_ref().and_then(AddRawMessageImap::new))
                    .with_add_raw_message_with_flags(|ctx| {
                        ctx.imap.as_ref().and_then(AddRawMessageWithFlagsImap::new)
                    });
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_add_raw_message(|ctx| {
                    ctx.notmuch.as_ref().and_then(AddRawMessageNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.peek_messages_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.maildir.as_ref().and_then(PeekMessagesMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(PeekMessagesMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_peek_messages(|ctx| ctx.imap.as_ref().and_then(PeekMessagesImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.notmuch.as_ref().and_then(PeekMessagesNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.get_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_get_messages(|ctx| ctx.imap.as_ref().and_then(GetMessagesImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_get_messages(|ctx| {
                    ctx.notmuch.as_ref().and_then(GetMessagesNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.copy_messages_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.maildir.as_ref().and_then(CopyMessagesMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(CopyMessagesMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_copy_messages(|ctx| ctx.imap.as_ref().and_then(CopyMessagesImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.notmuch.as_ref().and_then(CopyMessagesNotmuch::new)
                });
            }
            _ => (),
        }

        match toml_account_config.move_messages_kind() {
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.maildir.as_ref().and_then(MoveMessagesMaildir::new)
                });
            }
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(MoveMessagesMaildir::new)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_move_messages(|ctx| ctx.imap.as_ref().and_then(MoveMessagesImap::new));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.notmuch.as_ref().and_then(MoveMessagesNotmuch::new)
                });
            }
            _ => (),
        }

        Ok(Self {
            toml_account_config,
            builder: backend_builder,
        })
    }

    pub async fn build(self) -> Result<Backend> {
        Ok(Backend {
            toml_account_config: self.toml_account_config,
            backend: self.builder.build().await?,
        })
    }
}

impl Deref for BackendBuilder {
    type Target = email::backend::BackendBuilder<BackendContextBuilder>;

    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}

impl Into<email::backend::BackendBuilder<BackendContextBuilder>> for BackendBuilder {
    fn into(self) -> email::backend::BackendBuilder<BackendContextBuilder> {
        self.builder
    }
}

pub struct Backend {
    toml_account_config: TomlAccountConfig,
    backend: email::backend::Backend<BackendContext>,
}

impl Backend {
    pub async fn new(
        toml_account_config: TomlAccountConfig,
        account_config: AccountConfig,
        with_sending: bool,
    ) -> Result<Self> {
        BackendBuilder::new(toml_account_config, account_config, with_sending)
            .await?
            .build()
            .await
    }

    fn build_id_mapper(
        &self,
        folder: &str,
        backend_kind: Option<&BackendKind>,
    ) -> Result<IdMapper> {
        let mut id_mapper = IdMapper::Dummy;

        match backend_kind {
            Some(BackendKind::Maildir) => {
                if let Some(mdir_config) = &self.toml_account_config.maildir {
                    id_mapper = IdMapper::new(
                        &self.backend.account_config,
                        folder,
                        mdir_config.root_dir.clone(),
                    )?;
                }
            }
            Some(BackendKind::MaildirForSync) => {
                id_mapper = IdMapper::new(
                    &self.backend.account_config,
                    folder,
                    self.backend.account_config.get_sync_dir()?,
                )?;
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                if let Some(notmuch_config) = &self.toml_account_config.notmuch {
                    id_mapper = IdMapper::new(
                        &self.backend.account_config,
                        folder,
                        mdir_config.root_dir.clone(),
                    )?;
                }
            }
            _ => (),
        };

        Ok(id_mapper)
    }

    pub async fn list_envelopes(
        &self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        let backend_kind = self.toml_account_config.list_envelopes_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let envelopes = self.backend.list_envelopes(folder, page_size, page).await?;
        let envelopes = Envelopes::from_backend(&self.account_config, &id_mapper, envelopes)?;
        Ok(envelopes)
    }

    pub async fn add_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.add_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flags(folder, &ids, flags).await
    }

    pub async fn set_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.set_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.set_flags(folder, &ids, flags).await
    }

    pub async fn remove_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.remove_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.remove_flags(folder, &ids, flags).await
    }

    pub async fn peek_messages(&self, folder: &str, ids: &[usize]) -> Result<Messages> {
        let backend_kind = self.toml_account_config.get_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.peek_messages(folder, &ids).await
    }

    pub async fn get_messages(&self, folder: &str, ids: &[usize]) -> Result<Messages> {
        let backend_kind = self.toml_account_config.get_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.get_messages(folder, &ids).await
    }

    pub async fn copy_messages(
        &self,
        from_folder: &str,
        to_folder: &str,
        ids: &[usize],
    ) -> Result<()> {
        let backend_kind = self.toml_account_config.move_messages_kind();
        let id_mapper = self.build_id_mapper(from_folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend
            .copy_messages(from_folder, to_folder, &ids)
            .await
    }

    pub async fn move_messages(
        &self,
        from_folder: &str,
        to_folder: &str,
        ids: &[usize],
    ) -> Result<()> {
        let backend_kind = self.toml_account_config.move_messages_kind();
        let id_mapper = self.build_id_mapper(from_folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend
            .move_messages(from_folder, to_folder, &ids)
            .await
    }

    pub async fn delete_messages(&self, folder: &str, ids: &[usize]) -> Result<()> {
        let backend_kind = self.toml_account_config.delete_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.delete_messages(folder, &ids).await
    }

    pub async fn add_raw_message(&self, folder: &str, email: &[u8]) -> Result<SingleId> {
        let backend_kind = self.toml_account_config.add_raw_message_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let id = self.backend.add_raw_message(folder, email).await?;
        id_mapper.create_alias(&*id)?;
        Ok(id)
    }

    pub async fn add_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.add_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flag(folder, &ids, flag).await
    }

    pub async fn set_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.set_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.set_flag(folder, &ids, flag).await
    }

    pub async fn remove_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.remove_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.remove_flag(folder, &ids, flag).await
    }
}

impl Deref for Backend {
    type Target = email::backend::Backend<BackendContext>;

    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}
