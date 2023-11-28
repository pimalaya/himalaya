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
        envelope::{
            get::{imap::GetEnvelopeImap, maildir::GetEnvelopeMaildir},
            list::{imap::ListEnvelopesImap, maildir::ListEnvelopesMaildir},
        },
        flag::{
            add::{imap::AddFlagsImap, maildir::AddFlagsMaildir},
            remove::{imap::RemoveFlagsImap, maildir::RemoveFlagsMaildir},
            set::{imap::SetFlagsImap, maildir::SetFlagsMaildir},
        },
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
        },
    },
    folder::{
        add::{imap::AddFolderImap, maildir::AddFolderMaildir},
        delete::{imap::DeleteFolderImap, maildir::DeleteFolderMaildir},
        expunge::{imap::ExpungeFolderImap, maildir::ExpungeFolderMaildir},
        list::{imap::ListFoldersImap, maildir::ListFoldersMaildir},
        purge::imap::PurgeFolderImap,
    },
    maildir::{MaildirConfig, MaildirSessionBuilder, MaildirSessionSync},
    sendmail::SendmailContext,
};
use serde::{Deserialize, Serialize};

use crate::{account::DeserializedAccountConfig, config::DeserializedConfig};

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
    sync_cache: Option<MaildirSessionBuilder>,
    maildir: Option<MaildirSessionBuilder>,
    #[cfg(feature = "imap-backend")]
    imap: Option<ImapSessionBuilder>,
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

        if let Some(maildir) = self.sync_cache {
            ctx.sync_cache = Some(maildir.build().await?);
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
    pub sync_cache: Option<MaildirSessionSync>,
    pub maildir: Option<MaildirSessionSync>,
    #[cfg(feature = "imap-backend")]
    pub imap: Option<ImapSessionSync>,
    #[cfg(feature = "smtp-sender")]
    pub smtp: Option<SmtpClientSync>,
    pub sendmail: Option<SendmailContext>,
}

pub struct BackendBuilder(pub email::backend::BackendBuilder<BackendContextBuilder>);

pub type Backend = email::backend::Backend<BackendContext>;

impl BackendBuilder {
    pub async fn new(
        deserialized_account_config: DeserializedAccountConfig,
        account_config: AccountConfig,
        disable_cache: bool,
    ) -> Result<Self> {
        let backend_ctx_builder = BackendContextBuilder {
            maildir: deserialized_account_config
                .maildir
                .as_ref()
                .map(|mdir_config| {
                    MaildirSessionBuilder::new(account_config.clone(), mdir_config.clone())
                }),
            sync_cache: if account_config.sync && !disable_cache {
                Some(MaildirSessionBuilder::new(
                    account_config.clone(),
                    MaildirConfig {
                        root_dir: account_config.sync_dir()?,
                    },
                ))
            } else {
                None
            },
            #[cfg(feature = "imap-backend")]
            imap: deserialized_account_config
                .imap
                .as_ref()
                .map(|imap_config| {
                    ImapSessionBuilder::new(account_config.clone(), imap_config.clone())
                }),
            #[cfg(feature = "notmuch-backend")]
            notmuch: deserialized_account_config
                .notmuch
                .as_ref()
                .map(|notmuch_config| {
                    NotmuchSessionBuilder::new(account_config.clone(), notmuch_config.clone())
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
        };

        let mut backend_builder =
            email::backend::BackendBuilder::new(account_config.clone(), backend_ctx_builder);

        let add_folder = deserialized_account_config
            .folder
            .as_ref()
            .and_then(|folder| folder.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match add_folder {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.sync_cache.as_ref().and_then(AddFolderMaildir::new));
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.maildir.as_ref().and_then(AddFolderMaildir::new));
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.imap.as_ref().and_then(AddFolderImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.notmuch.as_ref().and_then(AddFolderNotmuch::new));
            }
            _ => (),
        }

        let list_folders = deserialized_account_config
            .folder
            .as_ref()
            .and_then(|folder| folder.list.as_ref())
            .and_then(|list| list.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match list_folders {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.sync_cache.as_ref().and_then(ListFoldersMaildir::new)
                });
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.maildir.as_ref().and_then(ListFoldersMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_list_folders(|ctx| ctx.imap.as_ref().and_then(ListFoldersImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.notmuch.as_ref().and_then(ListFoldersNotmuch::new)
                });
            }
            _ => (),
        }

        let expunge_folder = deserialized_account_config
            .folder
            .as_ref()
            .and_then(|folder| folder.expunge.as_ref())
            .and_then(|expunge| expunge.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match expunge_folder {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_expunge_folder(|ctx| {
                    ctx.sync_cache.as_ref().and_then(ExpungeFolderMaildir::new)
                });
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_expunge_folder(|ctx| {
                    ctx.maildir.as_ref().and_then(ExpungeFolderMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_expunge_folder(|ctx| ctx.imap.as_ref().and_then(ExpungeFolderImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_expunge_folder(|ctx| {
                    ctx.notmuch.as_ref().and_then(ExpungeFolderNotmuch::new)
                });
            }
            _ => (),
        }

        let purge_folder = deserialized_account_config
            .folder
            .as_ref()
            .and_then(|folder| folder.purge.as_ref())
            .and_then(|purge| purge.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match purge_folder {
            // TODO
            // Some(_) if account_config.sync && !disable_cache => {
            //     backend_builder = backend_builder.with_purge_folder(|ctx| {
            //         ctx.sync_cache.as_ref().and_then(PurgeFolderMaildir::new)
            //     });
            // }
            // TODO
            // Some(BackendKind::Maildir) => {
            //     backend_builder = backend_builder
            //         .with_purge_folder(|ctx| ctx.maildir.as_ref().and_then(PurgeFolderMaildir::new));
            // }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_purge_folder(|ctx| ctx.imap.as_ref().and_then(PurgeFolderImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_purge_folder(|ctx| {
                    ctx.notmuch.as_ref().and_then(PurgeFolderNotmuch::new)
                });
            }
            _ => (),
        }

        let delete_folder = deserialized_account_config
            .folder
            .as_ref()
            .and_then(|folder| folder.delete.as_ref())
            .and_then(|delete| delete.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match delete_folder {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_delete_folder(|ctx| {
                    ctx.sync_cache.as_ref().and_then(DeleteFolderMaildir::new)
                });
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_delete_folder(|ctx| {
                    ctx.maildir.as_ref().and_then(DeleteFolderMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_delete_folder(|ctx| ctx.imap.as_ref().and_then(DeleteFolderImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_delete_folder(|ctx| {
                    ctx.notmuch.as_ref().and_then(DeleteFolderNotmuch::new)
                });
            }
            _ => (),
        }

        let get_envelope = deserialized_account_config
            .envelope
            .as_ref()
            .and_then(|envelope| envelope.get.as_ref())
            .and_then(|get| get.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match get_envelope {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.sync_cache.as_ref().and_then(GetEnvelopeMaildir::new)
                });
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.maildir.as_ref().and_then(GetEnvelopeMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_get_envelope(|ctx| ctx.imap.as_ref().and_then(GetEnvelopeImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.notmuch.as_ref().and_then(GetEnvelopeNotmuch::new)
                });
            }
            _ => (),
        }

        let list_envelopes = deserialized_account_config
            .envelope
            .as_ref()
            .and_then(|envelope| envelope.list.as_ref())
            .and_then(|send| send.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match list_envelopes {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.sync_cache.as_ref().and_then(ListEnvelopesMaildir::new)
                });
            }
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

        let add_flags = deserialized_account_config
            .flag
            .as_ref()
            .and_then(|flag| flag.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match add_flags {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.sync_cache.as_ref().and_then(AddFlagsMaildir::new));
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.maildir.as_ref().and_then(AddFlagsMaildir::new));
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.imap.as_ref().and_then(AddFlagsImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.notmuch.as_ref().and_then(AddFlagsNotmuch::new));
            }
            _ => (),
        }

        let set_flags = deserialized_account_config
            .flag
            .as_ref()
            .and_then(|flag| flag.set.as_ref())
            .and_then(|set| set.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match set_flags {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.sync_cache.as_ref().and_then(SetFlagsMaildir::new));
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.maildir.as_ref().and_then(SetFlagsMaildir::new));
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.imap.as_ref().and_then(SetFlagsImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.notmuch.as_ref().and_then(SetFlagsNotmuch::new));
            }
            _ => (),
        }

        let remove_flags = deserialized_account_config
            .flag
            .as_ref()
            .and_then(|flag| flag.remove.as_ref())
            .and_then(|remove| remove.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match remove_flags {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.sync_cache.as_ref().and_then(RemoveFlagsMaildir::new)
                });
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.maildir.as_ref().and_then(RemoveFlagsMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_remove_flags(|ctx| ctx.imap.as_ref().and_then(RemoveFlagsImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.notmuch.as_ref().and_then(RemoveFlagsNotmuch::new)
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

        let add_msg = deserialized_account_config
            .message
            .as_ref()
            .and_then(|msg| msg.add.as_ref())
            .and_then(|add| add.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match add_msg {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_add_raw_message_with_flags(|ctx| {
                    ctx.sync_cache
                        .as_ref()
                        .and_then(AddRawMessageWithFlagsMaildir::new)
                });
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_add_raw_message_with_flags(|ctx| {
                    ctx.maildir
                        .as_ref()
                        .and_then(AddRawMessageWithFlagsMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_raw_message(|ctx| ctx.imap.as_ref().and_then(AddRawMessageImap::new))
                    .with_add_raw_message_with_flags(|ctx| {
                        ctx.imap.as_ref().and_then(AddRawMessageWithFlagsImap::new)
                    });
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_add_raw_message(|ctx| {
                    ctx.notmuch.as_ref().and_then(AddRawMessageNotmuch::new)
                });
            }
            _ => (),
        }

        let peek_msgs = deserialized_account_config
            .message
            .as_ref()
            .and_then(|msg| msg.peek.as_ref())
            .and_then(|peek| peek.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match peek_msgs {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.sync_cache.as_ref().and_then(PeekMessagesMaildir::new)
                });
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.maildir.as_ref().and_then(PeekMessagesMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_peek_messages(|ctx| ctx.imap.as_ref().and_then(PeekMessagesImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.notmuch.as_ref().and_then(PeekMessagesNotmuch::new)
                });
            }
            _ => (),
        }

        let get_msgs = deserialized_account_config
            .message
            .as_ref()
            .and_then(|msg| msg.get.as_ref())
            .and_then(|get| get.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match get_msgs {
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_get_messages(|ctx| ctx.imap.as_ref().and_then(GetMessagesImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_get_messages(|ctx| {
                    ctx.notmuch.as_ref().and_then(GetMessagesNotmuch::new)
                });
            }
            _ => (),
        }

        let copy_msgs = deserialized_account_config
            .message
            .as_ref()
            .and_then(|msg| msg.copy.as_ref())
            .and_then(|copy| copy.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match copy_msgs {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.sync_cache.as_ref().and_then(CopyMessagesMaildir::new)
                });
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.maildir.as_ref().and_then(CopyMessagesMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_copy_messages(|ctx| ctx.imap.as_ref().and_then(CopyMessagesImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.notmuch.as_ref().and_then(CopyMessagesNotmuch::new)
                });
            }
            _ => (),
        }

        let move_msgs = deserialized_account_config
            .message
            .as_ref()
            .and_then(|msg| msg.move_.as_ref())
            .and_then(|move_| move_.backend.as_ref())
            .or_else(|| deserialized_account_config.backend.as_ref());

        match move_msgs {
            Some(_) if account_config.sync && !disable_cache => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.sync_cache.as_ref().and_then(MoveMessagesMaildir::new)
                });
            }
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.maildir.as_ref().and_then(MoveMessagesMaildir::new)
                });
            }
            #[cfg(feature = "imap-backend")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_move_messages(|ctx| ctx.imap.as_ref().and_then(MoveMessagesImap::new));
            }
            #[cfg(feature = "notmuch-backend")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.notmuch.as_ref().and_then(MoveMessagesNotmuch::new)
                });
            }
            _ => (),
        }

        Ok(Self(backend_builder))
    }

    pub async fn build(self) -> Result<Backend> {
        self.0.build().await
    }
}
