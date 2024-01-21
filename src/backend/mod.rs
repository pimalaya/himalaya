pub mod config;
#[cfg(feature = "wizard")]
pub(crate) mod wizard;

use anyhow::Result;
use async_trait::async_trait;
use std::ops::Deref;

use email::account::config::AccountConfig;
#[cfg(all(feature = "envelope-get", feature = "imap"))]
use email::envelope::get::imap::GetImapEnvelope;
#[cfg(all(feature = "envelope-get", feature = "maildir"))]
use email::envelope::get::maildir::GetMaildirEnvelope;
#[cfg(all(feature = "envelope-get", feature = "notmuch"))]
use email::envelope::get::notmuch::GetNotmuchEnvelope;
#[cfg(all(feature = "envelope-list", feature = "imap"))]
use email::envelope::list::imap::ListImapEnvelopes;
#[cfg(all(feature = "envelope-list", feature = "maildir"))]
use email::envelope::list::maildir::ListMaildirEnvelopes;
#[cfg(all(feature = "envelope-list", feature = "notmuch"))]
use email::envelope::list::notmuch::ListNotmuchEnvelopes;
#[cfg(all(feature = "envelope-watch", feature = "imap"))]
use email::envelope::watch::imap::WatchImapEnvelopes;
#[cfg(all(feature = "envelope-watch", feature = "maildir"))]
use email::envelope::watch::maildir::WatchMaildirEnvelopes;
// #[cfg(all(feature = "envelope-watch", feature = "notmuch"))]
// use email::envelope::watch::notmuch::WatchNotmuchEnvelopes;
#[cfg(feature = "message-add")]
use email::envelope::SingleId;
#[cfg(all(feature = "flag-add", feature = "imap"))]
use email::flag::add::imap::AddImapFlags;
#[cfg(all(feature = "flag-add", feature = "maildir"))]
use email::flag::add::maildir::AddMaildirFlags;
#[cfg(all(feature = "flag-add", feature = "notmuch"))]
use email::flag::add::notmuch::AddNotmuchFlags;
#[cfg(all(feature = "flag-remove", feature = "imap"))]
use email::flag::remove::imap::RemoveImapFlags;
#[cfg(all(feature = "flag-remove", feature = "maildir"))]
use email::flag::remove::maildir::RemoveMaildirFlags;
#[cfg(all(feature = "flag-remove", feature = "notmuch"))]
use email::flag::remove::notmuch::RemoveNotmuchFlags;
#[cfg(all(feature = "flag-set", feature = "imap"))]
use email::flag::set::imap::SetImapFlags;
#[cfg(all(feature = "flag-set", feature = "maildir"))]
use email::flag::set::maildir::SetMaildirFlags;
#[cfg(all(feature = "flag-set", feature = "notmuch"))]
use email::flag::set::notmuch::SetNotmuchFlags;
#[cfg(all(feature = "folder-add", feature = "imap"))]
use email::folder::add::imap::AddImapFolder;
#[cfg(all(feature = "folder-add", feature = "maildir"))]
use email::folder::add::maildir::AddMaildirFolder;
#[cfg(all(feature = "folder-add", feature = "notmuch"))]
use email::folder::add::notmuch::AddNotmuchFolder;
#[cfg(all(feature = "folder-delete", feature = "imap"))]
use email::folder::delete::imap::DeleteImapFolder;
#[cfg(all(feature = "folder-delete", feature = "maildir"))]
use email::folder::delete::maildir::DeleteMaildirFolder;
// #[cfg(all(feature = "folder-delete", feature = "notmuch"))]
// use email::folder::delete::notmuch::DeleteNotmuchFolder;
#[cfg(all(feature = "folder-expunge", feature = "imap"))]
use email::folder::expunge::imap::ExpungeImapFolder;
#[cfg(all(feature = "folder-expunge", feature = "maildir"))]
use email::folder::expunge::maildir::ExpungeMaildirFolder;
// #[cfg(all(feature = "folder-expunge", feature = "notmuch"))]
// use email::folder::expunge::notmuch::ExpungeNotmuchFolder;
#[cfg(all(feature = "folder-list", feature = "imap"))]
use email::folder::list::imap::ListImapFolders;
#[cfg(all(feature = "folder-list", feature = "maildir"))]
use email::folder::list::maildir::ListMaildirFolders;
#[cfg(all(feature = "folder-list", feature = "notmuch"))]
use email::folder::list::notmuch::ListNotmuchFolders;
#[cfg(all(feature = "folder-purge", feature = "imap"))]
use email::folder::purge::imap::PurgeImapFolder;
// #[cfg(all(feature = "folder-purge", feature = "maildir"))]
// use email::folder::purge::maildir::PurgeMaildirFolder;
// #[cfg(all(feature = "folder-purge", feature = "notmuch"))]
// use email::folder::purge::notmuch::PurgeNotmuchFolder;
#[cfg(feature = "imap")]
use email::imap::{ImapContextBuilder, ImapContextSync};
#[cfg(feature = "account-sync")]
use email::maildir::config::MaildirConfig;
#[cfg(feature = "maildir")]
use email::maildir::{MaildirContextBuilder, MaildirContextSync};
#[cfg(all(feature = "message-add", feature = "imap"))]
use email::message::add::imap::AddImapMessage;
#[cfg(all(feature = "message-add", feature = "maildir"))]
use email::message::add::maildir::AddMaildirMessage;
#[cfg(all(feature = "message-add", feature = "notmuch"))]
use email::message::add::notmuch::AddNotmuchMessage;
#[cfg(all(feature = "message-copy", feature = "imap"))]
use email::message::copy::imap::CopyImapMessages;
#[cfg(all(feature = "message-copy", feature = "maildir"))]
use email::message::copy::maildir::CopyMaildirMessages;
#[cfg(all(feature = "message-copy", feature = "notmuch"))]
use email::message::copy::notmuch::CopyNotmuchMessages;
#[cfg(all(feature = "message-get", feature = "imap"))]
use email::message::get::imap::GetImapMessages;
#[cfg(all(feature = "message-move", feature = "imap"))]
use email::message::move_::imap::MoveImapMessages;
#[cfg(all(feature = "message-move", feature = "maildir"))]
use email::message::move_::maildir::MoveMaildirMessages;
#[cfg(all(feature = "message-move", feature = "notmuch"))]
use email::message::move_::notmuch::MoveNotmuchMessages;
#[cfg(all(feature = "message-peek", feature = "imap"))]
use email::message::peek::imap::PeekImapMessages;
#[cfg(all(feature = "message-peek", feature = "maildir"))]
use email::message::peek::maildir::PeekMaildirMessages;
#[cfg(all(feature = "message-peek", feature = "notmuch"))]
use email::message::peek::notmuch::PeekNotmuchMessages;
#[cfg(any(feature = "message-peek", feature = "message-get"))]
use email::message::Messages;
#[cfg(feature = "notmuch")]
use email::notmuch::{NotmuchContextBuilder, NotmuchContextSync};
#[cfg(feature = "sendmail")]
use email::sendmail::SendmailContext;
#[cfg(feature = "smtp")]
use email::smtp::{SmtpContextBuilder, SmtpContextSync};

#[allow(unused)]
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
    #[cfg(feature = "account-sync")]
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
            #[cfg(feature = "account-sync")]
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
    pub imap: Option<ImapContextBuilder>,
    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirContextBuilder>,
    #[cfg(feature = "account-sync")]
    pub maildir_for_sync: Option<MaildirContextBuilder>,
    #[cfg(feature = "notmuch")]
    pub notmuch: Option<NotmuchContextBuilder>,
    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpContextBuilder>,
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
                        ImapContextBuilder::new(account_config.clone(), imap_config.clone())
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
                    MaildirContextBuilder::new(account_config.clone(), mdir_config.clone())
                }),
            #[cfg(feature = "account-sync")]
            maildir_for_sync: Some(MaildirConfig {
                root_dir: account_config.get_sync_dir()?,
            })
            .filter(|_| kinds.contains(&&BackendKind::MaildirForSync))
            .map(|mdir_config| MaildirContextBuilder::new(account_config.clone(), mdir_config)),
            #[cfg(feature = "notmuch")]
            notmuch: toml_account_config
                .notmuch
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Notmuch))
                .map(|notmuch_config| {
                    NotmuchContextBuilder::new(account_config.clone(), notmuch_config.clone())
                }),
            #[cfg(feature = "smtp")]
            smtp: toml_account_config
                .smtp
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Smtp))
                .map(|smtp_config| {
                    SmtpContextBuilder::new(account_config.clone(), smtp_config.clone())
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

        #[cfg(feature = "account-sync")]
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
    pub imap: Option<ImapContextSync>,
    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirContextSync>,
    #[cfg(feature = "account-sync")]
    pub maildir_for_sync: Option<MaildirContextSync>,
    #[cfg(feature = "notmuch")]
    pub notmuch: Option<NotmuchContextSync>,
    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpContextSync>,
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
        #[cfg(feature = "account-sync")]
        let is_maildir_for_sync_used = used_backends.contains(&BackendKind::MaildirForSync);

        let backend_ctx_builder = BackendContextBuilder {
            #[cfg(feature = "imap")]
            imap: {
                let ctx_builder = toml_account_config
                    .imap
                    .as_ref()
                    .filter(|_| is_imap_used)
                    .map(|imap_config| {
                        ImapContextBuilder::new(account_config.clone(), imap_config.clone())
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
                    MaildirContextBuilder::new(account_config.clone(), mdir_config.clone())
                }),
            #[cfg(feature = "account-sync")]
            maildir_for_sync: Some(MaildirConfig {
                root_dir: account_config.get_sync_dir()?,
            })
            .filter(|_| is_maildir_for_sync_used)
            .map(|mdir_config| MaildirContextBuilder::new(account_config.clone(), mdir_config)),
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
                    .with_add_folder(|ctx| ctx.maildir.as_ref().map(AddMaildirFolder::new_boxed));
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_add_folder(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(AddMaildirFolder::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.imap.as_ref().map(AddImapFolder::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_add_folder(|ctx| ctx.notmuch.as_ref().map(AddNotmuchFolder::new_boxed));
            }
            _ => (),
        }

        #[cfg(feature = "folder-list")]
        match toml_account_config.list_folders_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.maildir.as_ref().map(ListMaildirFolders::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(ListMaildirFolders::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_list_folders(|ctx| ctx.imap.as_ref().map(ListImapFolders::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_list_folders(|ctx| {
                    ctx.notmuch.as_ref().map(ListNotmuchFolders::new_boxed)
                });
            }
            _ => (),
        }

        #[cfg(feature = "folder-expunge")]
        match toml_account_config.expunge_folder_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_expunge_folder(|ctx| {
                    ctx.maildir.as_ref().map(ExpungeMaildirFolder::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_expunge_folder(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(ExpungeMaildirFolder::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_expunge_folder(|ctx| ctx.imap.as_ref().map(ExpungeImapFolder::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                // TODO
                // backend_builder = backend_builder.with_expunge_folder(|ctx| {
                //     ctx.notmuch.as_ref().map(ExpungeNotmuchFolder::new_boxed)
                // });
            }
            _ => (),
        }

        #[cfg(feature = "folder-purge")]
        match toml_account_config.purge_folder_kind() {
            // TODO
            // #[cfg(feature = "maildir")]
            // Some(BackendKind::Maildir) => {
            //     backend_builder = backend_builder
            //         .with_purge_folder(|ctx| ctx.maildir.as_ref().map(PurgeMaildirFolder::new_boxed));
            // }
            // TODO
            // #[cfg(feature = "account-sync")]
            // Some(BackendKind::MaildirForSync) => {
            //     backend_builder = backend_builder
            //         .with_purge_folder(|ctx| ctx.maildir_for_sync.as_ref().map(PurgeMaildirFolder::new_boxed));
            // }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_purge_folder(|ctx| ctx.imap.as_ref().map(PurgeImapFolder::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                // TODO
                // backend_builder = backend_builder.with_purge_folder(|ctx| {
                //     ctx.notmuch.as_ref().map(PurgeNotmuchFolder::new_boxed)
                // });
            }
            _ => (),
        }

        #[cfg(feature = "folder-delete")]
        match toml_account_config.delete_folder_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_delete_folder(|ctx| {
                    ctx.maildir.as_ref().map(DeleteMaildirFolder::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_delete_folder(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(DeleteMaildirFolder::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_delete_folder(|ctx| ctx.imap.as_ref().map(DeleteImapFolder::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                // TODO
                // backend_builder = backend_builder.with_delete_folder(|ctx| {
                //     ctx.notmuch.as_ref().map(DeleteNotmuchFolder::new_boxed)
                // });
            }
            _ => (),
        }

        #[cfg(feature = "envelope-list")]
        match toml_account_config.list_envelopes_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.maildir.as_ref().map(ListMaildirEnvelopes::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(ListMaildirEnvelopes::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_list_envelopes(|ctx| ctx.imap.as_ref().map(ListImapEnvelopes::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_list_envelopes(|ctx| {
                    ctx.notmuch.as_ref().map(ListNotmuchEnvelopes::new_boxed)
                });
            }
            _ => (),
        }

        #[cfg(feature = "envelope-watch")]
        match toml_account_config.watch_envelopes_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_watch_envelopes(|ctx| {
                    ctx.maildir.as_ref().map(WatchMaildirEnvelopes::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_watch_envelopes(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(WatchMaildirEnvelopes::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder.with_watch_envelopes(|ctx| {
                    ctx.imap.as_ref().map(WatchImapEnvelopes::new_boxed)
                });
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                // TODO
                // backend_builder = backend_builder.with_watch_envelopes(|ctx| {
                //     ctx.notmuch.as_ref().map(WatchNotmuchEnvelopes::new_boxed)
                // });
            }
            _ => (),
        }

        #[cfg(feature = "envelope-get")]
        match toml_account_config.get_envelope_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.maildir.as_ref().map(GetMaildirEnvelope::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(GetMaildirEnvelope::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_get_envelope(|ctx| ctx.imap.as_ref().map(GetImapEnvelope::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_get_envelope(|ctx| {
                    ctx.notmuch.as_ref().map(GetNotmuchEnvelope::new_boxed)
                });
            }
            _ => (),
        }

        #[cfg(feature = "flag-add")]
        match toml_account_config.add_flags_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.maildir.as_ref().map(AddMaildirFlags::new_boxed));
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_add_flags(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(AddMaildirFlags::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.imap.as_ref().map(AddImapFlags::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_add_flags(|ctx| ctx.notmuch.as_ref().map(AddNotmuchFlags::new_boxed));
            }
            _ => (),
        }

        #[cfg(feature = "flag-set")]
        match toml_account_config.set_flags_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.maildir.as_ref().map(SetMaildirFlags::new_boxed));
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_set_flags(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(SetMaildirFlags::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.imap.as_ref().map(SetImapFlags::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_set_flags(|ctx| ctx.notmuch.as_ref().map(SetNotmuchFlags::new_boxed));
            }
            _ => (),
        }

        #[cfg(feature = "flag-remove")]
        match toml_account_config.remove_flags_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.maildir.as_ref().map(RemoveMaildirFlags::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(RemoveMaildirFlags::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_remove_flags(|ctx| ctx.imap.as_ref().map(RemoveImapFlags::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_remove_flags(|ctx| {
                    ctx.notmuch.as_ref().map(RemoveNotmuchFlags::new_boxed)
                });
            }
            _ => (),
        }

        #[cfg(feature = "message-add")]
        match toml_account_config.add_message_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_add_message(|ctx| ctx.imap.as_ref().map(AddImapMessage::new_boxed))
                    .with_add_message(|ctx| ctx.imap.as_ref().map(AddImapMessage::new_boxed));
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder
                    .with_add_message(|ctx| ctx.maildir.as_ref().map(AddMaildirMessage::new_boxed));
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_add_message(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(AddMaildirMessage::new_boxed)
                });
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder
                    .with_add_message(|ctx| ctx.notmuch.as_ref().map(AddNotmuchMessage::new_boxed));
            }
            _ => (),
        }

        #[cfg(feature = "message-peek")]
        match toml_account_config.peek_messages_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.maildir.as_ref().map(PeekMaildirMessages::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(PeekMaildirMessages::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_peek_messages(|ctx| ctx.imap.as_ref().map(PeekImapMessages::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_peek_messages(|ctx| {
                    ctx.notmuch.as_ref().map(PeekNotmuchMessages::new_boxed)
                });
            }
            _ => (),
        }

        #[cfg(feature = "message-get")]
        match toml_account_config.get_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_get_messages(|ctx| ctx.imap.as_ref().map(GetImapMessages::new_boxed));
            }
            _ => (),
        }

        #[cfg(feature = "message-copy")]
        match toml_account_config.copy_messages_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.maildir.as_ref().map(CopyMaildirMessages::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(CopyMaildirMessages::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_copy_messages(|ctx| ctx.imap.as_ref().map(CopyImapMessages::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_copy_messages(|ctx| {
                    ctx.notmuch.as_ref().map(CopyNotmuchMessages::new_boxed)
                });
            }
            _ => (),
        }

        #[cfg(feature = "message-move")]
        match toml_account_config.move_messages_kind() {
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.maildir.as_ref().map(MoveMaildirMessages::new_boxed)
                });
            }
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.maildir_for_sync
                        .as_ref()
                        .map(MoveMaildirMessages::new_boxed)
                });
            }
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => {
                backend_builder = backend_builder
                    .with_move_messages(|ctx| ctx.imap.as_ref().map(MoveImapMessages::new_boxed));
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                backend_builder = backend_builder.with_move_messages(|ctx| {
                    ctx.notmuch.as_ref().map(MoveNotmuchMessages::new_boxed)
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
impl From<BackendBuilder> for email::backend::BackendBuilder<BackendContextBuilder> {
    fn from(backend_builder: BackendBuilder) -> Self {
        backend_builder.builder
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
            #[cfg(feature = "account-sync")]
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
                        notmuch_config.get_maildir_path()?,
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
