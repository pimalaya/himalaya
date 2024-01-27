pub mod config;
#[cfg(feature = "wizard")]
pub(crate) mod wizard;

use anyhow::Result;
use async_trait::async_trait;
use std::{ops::Deref, sync::Arc};

#[cfg(any(feature = "account-sync", feature = "envelope-get"))]
use email::envelope::get::GetEnvelope;
#[cfg(any(feature = "account-sync", feature = "envelope-list"))]
use email::envelope::list::ListEnvelopes;
#[cfg(feature = "envelope-watch")]
use email::envelope::watch::WatchEnvelopes;
#[cfg(any(feature = "account-sync", feature = "flag-add"))]
use email::flag::add::AddFlags;
#[cfg(feature = "flag-remove")]
use email::flag::remove::RemoveFlags;
#[cfg(any(feature = "account-sync", feature = "flag-set"))]
use email::flag::set::SetFlags;
#[cfg(any(feature = "account-sync", feature = "folder-add"))]
use email::folder::add::AddFolder;
#[cfg(any(feature = "account-sync", feature = "folder-delete"))]
use email::folder::delete::DeleteFolder;
#[cfg(any(feature = "account-sync", feature = "folder-expunge"))]
use email::folder::expunge::ExpungeFolder;
#[cfg(any(feature = "account-sync", feature = "folder-list"))]
use email::folder::list::ListFolders;
#[cfg(feature = "folder-purge")]
use email::folder::purge::PurgeFolder;
#[cfg(feature = "imap")]
use email::imap::{ImapContextBuilder, ImapContextSync};
#[cfg(feature = "account-sync")]
use email::maildir::config::MaildirConfig;
#[cfg(any(feature = "account-sync", feature = "maildir"))]
use email::maildir::{MaildirContextBuilder, MaildirContextSync};
#[cfg(any(feature = "account-sync", feature = "message-add"))]
use email::message::add::AddMessage;
#[cfg(feature = "message-copy")]
use email::message::copy::CopyMessages;
#[cfg(feature = "message-delete")]
use email::message::delete::DeleteMessages;
#[cfg(any(feature = "account-sync", feature = "message-get"))]
use email::message::get::GetMessages;
#[cfg(any(feature = "account-sync", feature = "message-peek"))]
use email::message::peek::PeekMessages;
#[cfg(any(feature = "account-sync", feature = "message-move"))]
use email::message::r#move::MoveMessages;
#[cfg(feature = "message-send")]
use email::message::send::SendMessage;
#[cfg(feature = "notmuch")]
use email::notmuch::{NotmuchContextBuilder, NotmuchContextSync};
#[cfg(feature = "sendmail")]
use email::sendmail::{SendmailContextBuilder, SendmailContextSync};
#[cfg(feature = "smtp")]
use email::smtp::{SmtpContextBuilder, SmtpContextSync};
use email::{
    account::config::AccountConfig,
    backend::{
        macros::BackendContext, BackendFeatureBuilder, FindBackendSubcontext, MapBackendFeature,
    },
    envelope::{Id, SingleId},
    flag::{Flag, Flags},
    message::Messages,
};
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "account-sync", feature = "envelope-list"))]
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
    pub toml_account_config: Arc<TomlAccountConfig>,
    pub account_config: Arc<AccountConfig>,

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
    pub sendmail: Option<SendmailContextBuilder>,
}

impl BackendContextBuilder {
    pub async fn new(
        toml_account_config: Arc<TomlAccountConfig>,
        account_config: Arc<AccountConfig>,
        kinds: Vec<&BackendKind>,
    ) -> Result<Self> {
        Ok(Self {
            toml_account_config: toml_account_config.clone(),
            account_config: account_config.clone(),

            #[cfg(feature = "imap")]
            imap: {
                let builder = toml_account_config
                    .imap
                    .as_ref()
                    .filter(|_| kinds.contains(&&BackendKind::Imap))
                    .map(Clone::clone)
                    .map(Arc::new)
                    .map(|config| ImapContextBuilder::new(config).with_prebuilt_credentials());
                match builder {
                    Some(builder) => Some(builder.await?),
                    None => None,
                }
            },

            #[cfg(feature = "maildir")]
            maildir: toml_account_config
                .maildir
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Maildir))
                .map(Clone::clone)
                .map(Arc::new)
                .map(MaildirContextBuilder::new),

            #[cfg(feature = "account-sync")]
            maildir_for_sync: Some(MaildirConfig {
                root_dir: account_config.get_sync_dir()?,
            })
            .filter(|_| kinds.contains(&&BackendKind::MaildirForSync))
            .map(Arc::new)
            .map(MaildirContextBuilder::new),

            #[cfg(feature = "notmuch")]
            notmuch: toml_account_config
                .notmuch
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Notmuch))
                .map(Clone::clone)
                .map(Arc::new)
                .map(NotmuchContextBuilder::new),

            #[cfg(feature = "smtp")]
            smtp: toml_account_config
                .smtp
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Smtp))
                .map(Clone::clone)
                .map(Arc::new)
                .map(SmtpContextBuilder::new),

            #[cfg(feature = "sendmail")]
            sendmail: toml_account_config
                .sendmail
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Sendmail))
                .map(Clone::clone)
                .map(Arc::new)
                .map(SendmailContextBuilder::new),
        })
    }
}

#[async_trait]
impl email::backend::BackendContextBuilder for BackendContextBuilder {
    type Context = BackendContext;

    #[cfg(any(feature = "account-sync", feature = "folder-add"))]
    fn add_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn AddFolder> {
        match self.toml_account_config.add_folder_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.add_folder_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.add_folder_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.add_folder()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.add_folder_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "folder-list"))]
    fn list_folders(&self) -> BackendFeatureBuilder<Self::Context, dyn ListFolders> {
        match self.toml_account_config.list_folders_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.list_folders_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.list_folders_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.list_folders()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.list_folders_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "folder-expunge"))]
    fn expunge_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn ExpungeFolder> {
        match self.toml_account_config.expunge_folder_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.expunge_folder_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.expunge_folder_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.expunge_folder()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.expunge_folder_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(feature = "folder-purge")]
    fn purge_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn PurgeFolder> {
        match self.toml_account_config.purge_folder_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.purge_folder_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.purge_folder_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.purge_folder()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.purge_folder_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "folder-delete"))]
    fn delete_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn DeleteFolder> {
        match self.toml_account_config.delete_folder_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.delete_folder_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.delete_folder_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.delete_folder()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.delete_folder_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "envelope-list"))]
    fn list_envelopes(&self) -> BackendFeatureBuilder<Self::Context, dyn ListEnvelopes> {
        match self.toml_account_config.list_envelopes_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.list_envelopes_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.list_envelopes_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.list_envelopes()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.list_envelopes_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(feature = "envelope-watch")]
    fn watch_envelopes(&self) -> BackendFeatureBuilder<Self::Context, dyn WatchEnvelopes> {
        match self.toml_account_config.watch_envelopes_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.watch_envelopes_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.watch_envelopes_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.watch_envelopes()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.watch_envelopes_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "envelope-get"))]
    fn get_envelope(&self) -> BackendFeatureBuilder<Self::Context, dyn GetEnvelope> {
        match self.toml_account_config.get_envelope_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.get_envelope_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.get_envelope_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.get_envelope()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.get_envelope_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "flag-add"))]
    fn add_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn AddFlags> {
        match self.toml_account_config.add_flags_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.add_flags_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.add_flags_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.add_flags()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.add_flags_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "flag-set"))]
    fn set_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn SetFlags> {
        match self.toml_account_config.set_flags_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.set_flags_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.set_flags_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.set_flags()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.set_flags_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(feature = "flag-remove")]
    fn remove_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn RemoveFlags> {
        match self.toml_account_config.remove_flags_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.remove_flags_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.remove_flags_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.remove_flags()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.remove_flags_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "message-add"))]
    fn add_message(&self) -> BackendFeatureBuilder<Self::Context, dyn AddMessage> {
        match self.toml_account_config.add_message_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.add_message_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.add_message_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.add_message()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.add_message_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(feature = "message-send")]
    fn send_message(&self) -> BackendFeatureBuilder<Self::Context, dyn SendMessage> {
        match self.toml_account_config.send_message_kind() {
            #[cfg(feature = "smtp")]
            Some(BackendKind::Smtp) => self.send_message_from(self.smtp.as_ref()),
            #[cfg(feature = "sendmail")]
            Some(BackendKind::Sendmail) => self.send_message_from(self.sendmail.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "message-peek"))]
    fn peek_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn PeekMessages> {
        match self.toml_account_config.peek_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.peek_messages_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.peek_messages_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.peek_messages()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.peek_messages_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "message-get"))]
    fn get_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn GetMessages> {
        match self.toml_account_config.get_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.get_messages_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.get_messages_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.get_messages()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.get_messages_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(feature = "message-copy")]
    fn copy_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn CopyMessages> {
        match self.toml_account_config.copy_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.copy_messages_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.copy_messages_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.copy_messages()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.copy_messages_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(any(feature = "account-sync", feature = "message-move"))]
    fn move_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn MoveMessages> {
        match self.toml_account_config.move_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.move_messages_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.move_messages_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.move_messages()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.move_messages_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    #[cfg(feature = "message-delete")]
    fn delete_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn DeleteMessages> {
        match self.toml_account_config.delete_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.delete_messages_from(self.imap.as_ref()),
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.delete_messages_from(self.maildir.as_ref()),
            #[cfg(feature = "account-sync")]
            Some(BackendKind::MaildirForSync) => {
                let f = self.maildir_for_sync.as_ref()?.delete_messages()?;
                Some(Arc::new(move |ctx| f(ctx.maildir_for_sync.as_ref()?)))
            }
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.delete_messages_from(self.notmuch.as_ref()),
            _ => None,
        }
    }

    async fn build(self, config: Arc<AccountConfig>) -> Result<Self::Context> {
        let mut ctx = BackendContext::default();

        #[cfg(feature = "imap")]
        if let Some(imap) = self.imap {
            ctx.imap = Some(imap.build(config.clone()).await?);
        }

        #[cfg(feature = "maildir")]
        if let Some(maildir) = self.maildir {
            ctx.maildir = Some(maildir.build(config.clone()).await?);
        }

        #[cfg(feature = "account-sync")]
        if let Some(maildir) = self.maildir_for_sync {
            ctx.maildir_for_sync = Some(maildir.build(config.clone()).await?);
        }

        #[cfg(feature = "notmuch")]
        if let Some(notmuch) = self.notmuch {
            ctx.notmuch = Some(notmuch.build(config.clone()).await?);
        }

        #[cfg(feature = "smtp")]
        if let Some(smtp) = self.smtp {
            ctx.smtp = Some(smtp.build(config.clone()).await?);
        }

        #[cfg(feature = "sendmail")]
        if let Some(sendmail) = self.sendmail {
            ctx.sendmail = Some(sendmail.build(config.clone()).await?);
        }

        Ok(ctx)
    }
}

#[derive(BackendContext, Default)]
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
    pub sendmail: Option<SendmailContextSync>,
}

#[cfg(feature = "imap")]
impl FindBackendSubcontext<ImapContextSync> for BackendContext {
    fn find_subcontext(&self) -> Option<&ImapContextSync> {
        self.imap.as_ref()
    }
}

#[cfg(feature = "maildir")]
impl FindBackendSubcontext<MaildirContextSync> for BackendContext {
    fn find_subcontext(&self) -> Option<&MaildirContextSync> {
        self.maildir.as_ref()
    }
}

#[cfg(feature = "notmuch")]
impl FindBackendSubcontext<NotmuchContextSync> for BackendContext {
    fn find_subcontext(&self) -> Option<&NotmuchContextSync> {
        self.notmuch.as_ref()
    }
}

#[cfg(feature = "smtp")]
impl FindBackendSubcontext<SmtpContextSync> for BackendContext {
    fn find_subcontext(&self) -> Option<&SmtpContextSync> {
        self.smtp.as_ref()
    }
}

#[cfg(feature = "sendmail")]
impl FindBackendSubcontext<SendmailContextSync> for BackendContext {
    fn find_subcontext(&self) -> Option<&SendmailContextSync> {
        self.sendmail.as_ref()
    }
}

pub struct Backend {
    pub toml_account_config: Arc<TomlAccountConfig>,
    pub backend: email::backend::Backend<BackendContext>,
}

impl Backend {
    pub async fn new(
        toml_account_config: Arc<TomlAccountConfig>,
        account_config: Arc<AccountConfig>,
        backend_kinds: impl IntoIterator<Item = &BackendKind>,
        with_features: impl Fn(&mut email::backend::BackendBuilder<BackendContextBuilder>),
    ) -> Result<Self> {
        let backend_kinds = backend_kinds.into_iter().collect();
        let backend_ctx_builder = BackendContextBuilder::new(
            toml_account_config.clone(),
            account_config.clone(),
            backend_kinds,
        )
        .await?;
        let mut backend_builder =
            email::backend::BackendBuilder::new(account_config.clone(), backend_ctx_builder)
                .with_default_features_disabled();

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

    #[cfg(any(feature = "account-sync", feature = "envelope-list"))]
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

    #[cfg(any(feature = "account-sync", feature = "flag-add"))]
    pub async fn add_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.add_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flags(folder, &ids, flags).await
    }

    #[cfg(any(feature = "account-sync", feature = "flag-add"))]
    pub async fn add_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.add_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flag(folder, &ids, flag).await
    }

    #[cfg(any(feature = "account-sync", feature = "flag-set"))]
    pub async fn set_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.set_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.set_flags(folder, &ids, flags).await
    }

    #[cfg(any(feature = "account-sync", feature = "flag-set"))]
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

    #[cfg(any(feature = "account-sync", feature = "message-add"))]
    pub async fn add_message(&self, folder: &str, email: &[u8]) -> Result<SingleId> {
        let backend_kind = self.toml_account_config.add_message_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let id = self.backend.add_message(folder, email).await?;
        id_mapper.create_alias(&*id)?;
        Ok(id)
    }

    #[cfg(any(feature = "account-sync", feature = "message-peek"))]
    pub async fn peek_messages(&self, folder: &str, ids: &[usize]) -> Result<Messages> {
        let backend_kind = self.toml_account_config.get_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.peek_messages(folder, &ids).await
    }

    #[cfg(any(feature = "account-sync", feature = "message-get"))]
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

    #[cfg(any(feature = "account-sync", feature = "message-move"))]
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
