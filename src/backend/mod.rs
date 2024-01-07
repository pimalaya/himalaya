pub mod config;
pub(crate) mod wizard;

use anyhow::Result;
use async_trait::async_trait;
use std::ops::Deref;

use email::account::config::AccountConfig;
#[cfg(all(feature = "envelope-get", feature = "imap"))]
use email::envelope::get::imap::GetEnvelopeImap;
#[cfg(all(feature = "envelope-get", feature = "maildir"))]
use email::envelope::get::maildir::GetEnvelopeMaildir;
#[cfg(all(feature = "envelope-list", feature = "imap"))]
use email::envelope::list::imap::ListEnvelopesImap;
#[cfg(all(feature = "envelope-list", feature = "maildir"))]
use email::envelope::list::maildir::ListEnvelopesMaildir;
#[cfg(all(feature = "envelope-watch", feature = "imap"))]
use email::envelope::watch::imap::WatchImapEnvelopes;
#[cfg(all(feature = "envelope-watch", feature = "maildir"))]
use email::envelope::watch::maildir::WatchMaildirEnvelopes;
#[cfg(feature = "message-add")]
use email::envelope::SingleId;
#[cfg(all(feature = "flag-add", feature = "imap"))]
use email::flag::add::imap::AddFlagsImap;
#[cfg(all(feature = "flag-add", feature = "maildir"))]
use email::flag::add::maildir::AddFlagsMaildir;
#[cfg(all(feature = "flag-remove", feature = "imap"))]
use email::flag::remove::imap::RemoveFlagsImap;
#[cfg(all(feature = "flag-remove", feature = "maildir"))]
use email::flag::remove::maildir::RemoveFlagsMaildir;
#[cfg(all(feature = "flag-set", feature = "imap"))]
use email::flag::set::imap::SetFlagsImap;
#[cfg(all(feature = "flag-set", feature = "maildir"))]
use email::flag::set::maildir::SetFlagsMaildir;
#[cfg(all(feature = "folder-add", feature = "imap"))]
use email::folder::add::imap::AddFolderImap;
#[cfg(all(feature = "folder-add", feature = "maildir"))]
use email::folder::add::maildir::AddFolderMaildir;
#[cfg(all(feature = "folder-delete", feature = "imap"))]
use email::folder::delete::imap::DeleteFolderImap;
#[cfg(all(feature = "folder-delete", feature = "maildir"))]
use email::folder::delete::maildir::DeleteFolderMaildir;
#[cfg(all(feature = "folder-expunge", feature = "imap"))]
use email::folder::expunge::imap::ExpungeFolderImap;
#[cfg(all(feature = "folder-expunge", feature = "maildir"))]
use email::folder::expunge::maildir::ExpungeFolderMaildir;
#[cfg(all(feature = "folder-list", feature = "imap"))]
use email::folder::list::imap::ListFoldersImap;
#[cfg(all(feature = "folder-list", feature = "maildir"))]
use email::folder::list::maildir::ListFoldersMaildir;
#[cfg(all(feature = "folder-purge", feature = "imap"))]
use email::folder::purge::imap::PurgeFolderImap;
#[cfg(feature = "imap")]
use email::imap::{ImapSessionBuilder, ImapSessionSync};
#[cfg(feature = "sync")]
use email::maildir::config::MaildirConfig;
#[cfg(feature = "maildir")]
use email::maildir::{MaildirSessionBuilder, MaildirSessionSync};
#[cfg(all(feature = "message-add", feature = "maildir"))]
use email::message::add_with_flags::maildir::AddMessageWithFlagsMaildir;
#[cfg(all(feature = "message-copy", feature = "imap"))]
use email::message::copy::imap::CopyMessagesImap;
#[cfg(all(feature = "message-copy", feature = "maildir"))]
use email::message::copy::maildir::CopyMessagesMaildir;
#[cfg(all(feature = "message-get", feature = "imap"))]
use email::message::get::imap::GetMessagesImap;
#[cfg(all(feature = "message-move", feature = "imap"))]
use email::message::move_::imap::MoveMessagesImap;
#[cfg(all(feature = "message-move", feature = "maildir"))]
use email::message::move_::maildir::MoveMessagesMaildir;
#[cfg(all(feature = "message-peek", feature = "imap"))]
use email::message::peek::imap::PeekMessagesImap;
#[cfg(all(feature = "message-peek", feature = "maildir"))]
use email::message::peek::maildir::PeekMessagesMaildir;
#[cfg(any(feature = "message-peek", feature = "message-get"))]
use email::message::Messages;
#[cfg(all(feature = "message-add", feature = "imap"))]
use email::message::{add::imap::AddMessageImap, add_with_flags::imap::AddMessageWithFlagsImap};
#[cfg(feature = "sendmail")]
use email::sendmail::SendmailContext;
#[cfg(feature = "smtp")]
use email::smtp::{SmtpClientBuilder, SmtpClientSync};

#[cfg(any(feature = "flag-command"))]
use email::{
    envelope::Id,
    flag::{Flag, Flags},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "envelope-list")]
use crate::envelope::Envelopes;
use crate::{account::config::TomlAccountConfig, cache::IdMapper};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    #[cfg(feature = "imap")]
    Imap,
    #[cfg(feature = "maildir")]
    Maildir,
    #[cfg(feature = "sync")]
    #[serde(skip_deserializing)]
    MaildirForSync,
    #[cfg(feature = "notmuch")]
    Notmuch,
    #[cfg(feature = "smtp")]
    Smtp,
    #[cfg(feature = "sendmail")]
    Sendmail,
    None,
}

impl ToString for BackendKind {
    fn to_string(&self) -> String {
        let kind = match self {
            #[cfg(feature = "imap")]
            Self::Imap => "IMAP",
            #[cfg(feature = "maildir")]
            Self::Maildir => "Maildir",
            #[cfg(feature = "sync")]
            Self::MaildirForSync => "Maildir",
            #[cfg(feature = "notmuch")]
            Self::Notmuch => "Notmuch",
            #[cfg(feature = "smtp")]
            Self::Smtp => "SMTP",
            #[cfg(feature = "sendmail")]
            Self::Sendmail => "Sendmail",
            Self::None => "None",
        };

        kind.to_string()
    }
}

#[derive(Clone, Default)]
pub struct BackendContextBuilder {
    #[cfg(feature = "imap")]
    pub imap: Option<ImapSessionBuilder>,
    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirSessionBuilder>,
    #[cfg(feature = "sync")]
    pub maildir_for_sync: Option<MaildirSessionBuilder>,
    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpClientBuilder>,
    #[cfg(feature = "sendmail")]
    pub sendmail: Option<SendmailContext>,
}

impl BackendContextBuilder {
    #[allow(unused)]
    pub async fn new(
        toml_account_config: &TomlAccountConfig,
        account_config: &AccountConfig,
        kinds: Vec<&BackendKind>,
    ) -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "imap")]
            imap: {
                let ctx_builder = toml_account_config
                    .imap
                    .as_ref()
                    .filter(|_| kinds.contains(&&BackendKind::Imap))
                    .map(|imap_config| {
                        ImapSessionBuilder::new(account_config.clone(), imap_config.clone())
                            .with_prebuilt_credentials()
                    });
                match ctx_builder {
                    Some(ctx_builder) => Some(ctx_builder.await?),
                    None => None,
                }
            },
            #[cfg(feature = "maildir")]
            maildir: toml_account_config
                .maildir
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Maildir))
                .map(|mdir_config| {
                    MaildirSessionBuilder::new(account_config.clone(), mdir_config.clone())
                }),
            #[cfg(feature = "sync")]
            maildir_for_sync: Some(MaildirConfig {
                root_dir: account_config.get_sync_dir()?,
            })
            .filter(|_| kinds.contains(&&BackendKind::MaildirForSync))
            .map(|mdir_config| MaildirSessionBuilder::new(account_config.clone(), mdir_config)),
            #[cfg(feature = "notmuch")]
            notmuch: toml_account_config
                .notmuch
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Notmuch))
                .map(|notmuch_config| {
                    NotmuchSessionBuilder::new(account_config.clone(), notmuch_config.clone())
                }),
            #[cfg(feature = "smtp")]
            smtp: toml_account_config
                .smtp
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Smtp))
                .map(|smtp_config| {
                    SmtpClientBuilder::new(account_config.clone(), smtp_config.clone())
                }),
            #[cfg(feature = "sendmail")]
            sendmail: toml_account_config
                .sendmail
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Sendmail))
                .map(|sendmail_config| {
                    SendmailContext::new(account_config.clone(), sendmail_config.clone())
                }),
        })
    }
}

#[async_trait]
impl email::backend::BackendContextBuilder for BackendContextBuilder {
    type Context = BackendContext;

    async fn build(self) -> Result<Self::Context> {
        #[allow(unused_mut)]
        let mut ctx = BackendContext::default();

        #[cfg(feature = "imap")]
        if let Some(imap) = self.imap {
            ctx.imap = Some(imap.build().await?);
        }

        #[cfg(feature = "maildir")]
        if let Some(maildir) = self.maildir {
            ctx.maildir = Some(maildir.build().await?);
        }

        #[cfg(feature = "sync")]
        if let Some(maildir) = self.maildir_for_sync {
            ctx.maildir_for_sync = Some(maildir.build().await?);
        }

        #[cfg(feature = "notmuch")]
        if let Some(notmuch) = self.notmuch {
            ctx.notmuch = Some(notmuch.build().await?);
        }

        #[cfg(feature = "smtp")]
        if let Some(smtp) = self.smtp {
            ctx.smtp = Some(smtp.build().await?);
        }

        #[cfg(feature = "sendmail")]
        if let Some(sendmail) = self.sendmail {
            ctx.sendmail = Some(sendmail.build().await?);
        }

        Ok(ctx)
    }
}

#[derive(Default)]
pub struct BackendContext {
    #[cfg(feature = "imap")]
    pub imap: Option<ImapSessionSync>,
    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirSessionSync>,
    #[cfg(feature = "sync")]
    pub maildir_for_sync: Option<MaildirSessionSync>,
    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpClientSync>,
    #[cfg(feature = "sendmail")]
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
    ) -> Result<Self> {
        #[allow(unused)]
        let used_backends = toml_account_config.get_used_backends();

        #[cfg(feature = "imap")]
        let is_imap_used = used_backends.contains(&BackendKind::Imap);
        #[cfg(feature = "maildir")]
        let is_maildir_used = used_backends.contains(&BackendKind::Maildir);
        #[cfg(feature = "sync")]
        let is_maildir_for_sync_used = used_backends.contains(&BackendKind::MaildirForSync);

        let backend_ctx_builder = BackendContextBuilder {
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
            #[cfg(feature = "maildir")]
            maildir: toml_account_config
                .maildir
                .as_ref()
                .filter(|_| is_maildir_used)
                .map(|mdir_config| {
                    MaildirSessionBuilder::new(account_config.clone(), mdir_config.clone())
                }),
            #[cfg(feature = "sync")]
            maildir_for_sync: Some(MaildirConfig {
                root_dir: account_config.get_sync_dir()?,
            })
            .filter(|_| is_maildir_for_sync_used)
            .map(|mdir_config| MaildirSessionBuilder::new(account_config.clone(), mdir_config)),
            ..Default::default()
        };

        #[allow(unused_mut)]
        let mut backend_builder =
            email::backend::BackendBuilder::new(account_config.clone(), backend_ctx_builder);

        #[cfg(feature = "folder-add")]
        match toml_account_config.add_folder_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.maildir.as_ref().and_then(AddFolderMaildir::new));
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "folder-list")]
        match toml_account_config.list_folders_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.maildir.as_ref().and_then(ListFoldersMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "folder-expunge")]
        match toml_account_config.expunge_folder_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_expunge_folder(|ctx| {
                    ctx.maildir.as_ref().and_then(ExpungeFolderMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "folder-purge")]
        match toml_account_config.purge_folder_kind() {
            // TODO
            // #[cfg(feature = "maildir")]
            // Some(BackendKind::Maildir) => {
            //     backend_builder = backend_builder
            //         .with_purge_folder(|ctx| ctx.maildir.as_ref().and_then(PurgeFolderMaildir::new));
            // }
            // TODO
            // #[cfg(feature = "sync")]
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

        #[cfg(feature = "folder-delete")]
        match toml_account_config.delete_folder_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_delete_folder(|ctx| {
                    ctx.maildir.as_ref().and_then(DeleteFolderMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "envelope-list")]
        match toml_account_config.list_envelopes_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.maildir.as_ref().and_then(ListEnvelopesMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "envelope-watch")]
        match toml_account_config.watch_envelopes_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_watch_envelopes(|ctx| {
                    ctx.maildir.as_ref().and_then(WatchMaildirEnvelopes::new)
                });
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "envelope-get")]
        match toml_account_config.get_envelope_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.maildir.as_ref().and_then(GetEnvelopeMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "flag-add")]
        match toml_account_config.add_flags_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.maildir.as_ref().and_then(AddFlagsMaildir::new));
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "flag-set")]
        match toml_account_config.set_flags_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.maildir.as_ref().and_then(SetFlagsMaildir::new));
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "flag-remove")]
        match toml_account_config.remove_flags_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.maildir.as_ref().and_then(RemoveFlagsMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "message-add")]
        match toml_account_config.add_message_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_message(|ctx| ctx.imap.as_ref().and_then(AddMessageImap::new))
                    .with_add_message_with_flags(|ctx| {
                        ctx.imap.as_ref().and_then(AddMessageWithFlagsImap::new)
                    });
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_add_message_with_flags(|ctx| {
                    ctx.maildir
                        .as_ref()
                        .and_then(AddMessageWithFlagsMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_add_message_with_flags(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .and_then(AddMessageWithFlagsMaildir::new)
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

        #[cfg(feature = "message-peek")]
        match toml_account_config.peek_messages_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.maildir.as_ref().and_then(PeekMessagesMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "message-get")]
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

        #[cfg(feature = "message-copy")]
        match toml_account_config.copy_messages_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.maildir.as_ref().and_then(CopyMessagesMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
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

        #[cfg(feature = "message-move")]
        match toml_account_config.move_messages_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.maildir.as_ref().and_then(MoveMessagesMaildir::new)
                });
            }
            #[cfg(feature = "sync")]
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
    #[allow(unused)]
    toml_account_config: TomlAccountConfig,
    backend: email::backend::Backend<BackendContext>,
}

impl Backend {
    pub async fn new(
        toml_account_config: &TomlAccountConfig,
        account_config: &AccountConfig,
        backend_kinds: impl IntoIterator<Item = &BackendKind>,
        with_features: impl Fn(&mut email::backend::BackendBuilder<BackendContextBuilder>),
    ) -> Result<Self> {
        let backend_kinds = backend_kinds.into_iter().collect();
        let backend_ctx_builder =
            BackendContextBuilder::new(toml_account_config, account_config, backend_kinds).await?;
        let mut backend_builder =
            email::backend::BackendBuilder::new(account_config.clone(), backend_ctx_builder);

        with_features(&mut backend_builder);

        Ok(Self {
            toml_account_config: toml_account_config.clone(),
            backend: backend_builder.build().await?,
        })
    }

    #[allow(unused)]
    fn build_id_mapper(
        &self,
        folder: &str,
        backend_kind: Option<&BackendKind>,
    ) -> Result<IdMapper> {
        #[allow(unused_mut)]
        let mut id_mapper = IdMapper::Dummy;

        match backend_kind {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                if let Some(mdir_config) = &self.toml_account_config.maildir {
                    id_mapper = IdMapper::new(
                        &self.backend.account_config,
                        folder,
                        mdir_config.root_dir.clone(),
                    )?;
                }
            }
            #[cfg(feature = "sync")]
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

    #[cfg(feature = "envelope-list")]
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

    #[cfg(feature = "flag-add")]
    pub async fn add_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.add_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flags(folder, &ids, flags).await
    }

    #[cfg(feature = "flag-add")]
    pub async fn add_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.add_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flag(folder, &ids, flag).await
    }

    #[cfg(feature = "flag-set")]
    pub async fn set_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.set_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.set_flags(folder, &ids, flags).await
    }

    #[cfg(feature = "flag-set")]
    pub async fn set_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.set_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.set_flag(folder, &ids, flag).await
    }

    #[cfg(feature = "flag-remove")]
    pub async fn remove_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.remove_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.remove_flags(folder, &ids, flags).await
    }

    #[cfg(feature = "flag-remove")]
    pub async fn remove_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.remove_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.remove_flag(folder, &ids, flag).await
    }

    #[cfg(feature = "message-add")]
    pub async fn add_message(&self, folder: &str, email: &[u8]) -> Result<SingleId> {
        let backend_kind = self.toml_account_config.add_message_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let id = self.backend.add_message(folder, email).await?;
        id_mapper.create_alias(&*id)?;
        Ok(id)
    }

    #[cfg(feature = "message-peek")]
    pub async fn peek_messages(&self, folder: &str, ids: &[usize]) -> Result<Messages> {
        let backend_kind = self.toml_account_config.get_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.peek_messages(folder, &ids).await
    }

    #[cfg(feature = "message-get")]
    pub async fn get_messages(&self, folder: &str, ids: &[usize]) -> Result<Messages> {
        let backend_kind = self.toml_account_config.get_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.get_messages(folder, &ids).await
    }

    #[cfg(feature = "message-copy")]
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

    #[cfg(feature = "message-move")]
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

    #[cfg(feature = "message-delete")]
    pub async fn delete_messages(&self, folder: &str, ids: &[usize]) -> Result<()> {
        let backend_kind = self.toml_account_config.delete_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.delete_messages(folder, &ids).await
    }
}

impl Deref for Backend {
    type Target = email::backend::Backend<BackendContext>;

    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}
