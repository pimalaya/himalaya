pub mod config;
pub(crate) mod wizard;

use color_eyre::Result;
use async_trait::async_trait;
use std::{ops::Deref, sync::Arc};

#[cfg(feature = "imap")]
use email::imap::{ImapContextBuilder, ImapContextSync};
#[cfg(any(feature = "account-sync", feature = "maildir"))]
use email::maildir::{MaildirContextBuilder, MaildirContextSync};
#[cfg(feature = "notmuch")]
use email::notmuch::{NotmuchContextBuilder, NotmuchContextSync};
#[cfg(feature = "sendmail")]
use email::sendmail::{SendmailContextBuilder, SendmailContextSync};
#[cfg(feature = "smtp")]
use email::smtp::{SmtpContextBuilder, SmtpContextSync};
use email::{
    account::config::AccountConfig,
    backend::{
        feature::BackendFeature, macros::BackendContext, mapper::SomeBackendContextBuilderMapper,
    },
    envelope::{
        get::GetEnvelope,
        list::{ListEnvelopes, ListEnvelopesOptions},
        watch::WatchEnvelopes,
        Id, SingleId,
    },
    flag::{add::AddFlags, remove::RemoveFlags, set::SetFlags, Flag, Flags},
    folder::{
        add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders,
        purge::PurgeFolder,
    },
    message::{
        add::AddMessage,
        copy::CopyMessages,
        delete::DeleteMessages,
        get::GetMessages,
        peek::PeekMessages,
        r#move::MoveMessages,
        send::{SendMessage, SendMessageThenSaveCopy},
        Messages,
    },
    AnyResult,
};
use serde::{Deserialize, Serialize};

use crate::{account::config::TomlAccountConfig, cache::IdMapper, envelope::Envelopes};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    None,

    #[cfg(feature = "imap")]
    Imap,
    #[cfg(all(feature = "imap", feature = "account-sync"))]
    ImapCache,

    #[cfg(feature = "maildir")]
    Maildir,

    #[cfg(feature = "notmuch")]
    Notmuch,

    #[cfg(feature = "smtp")]
    Smtp,

    #[cfg(feature = "sendmail")]
    Sendmail,
}

impl ToString for BackendKind {
    fn to_string(&self) -> String {
        let kind = match self {
            Self::None => "None",

            #[cfg(feature = "imap")]
            Self::Imap => "IMAP",
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Self::ImapCache => "IMAP cache",

            #[cfg(feature = "maildir")]
            Self::Maildir => "Maildir",

            #[cfg(feature = "notmuch")]
            Self::Notmuch => "Notmuch",

            #[cfg(feature = "smtp")]
            Self::Smtp => "SMTP",

            #[cfg(feature = "sendmail")]
            Self::Sendmail => "Sendmail",
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

    #[cfg(all(feature = "imap", feature = "account-sync"))]
    pub imap_cache: Option<MaildirContextBuilder>,

    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirContextBuilder>,

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
                    .map(|imap_config| {
                        ImapContextBuilder::new(account_config.clone(), imap_config)
                            .with_prebuilt_credentials()
                    });
                match builder {
                    Some(builder) => Some(builder.await?),
                    None => None,
                }
            },

            #[cfg(all(feature = "imap", feature = "account-sync"))]
            imap_cache: {
                let builder = toml_account_config
                    .imap
                    .as_ref()
                    .filter(|_| kinds.contains(&&BackendKind::ImapCache))
                    .map(Clone::clone)
                    .map(Arc::new)
                    .map(|imap_config| {
                        email::backend::context::BackendContextBuilder::try_to_sync_cache_builder(
                            &ImapContextBuilder::new(account_config.clone(), imap_config),
                            &account_config,
                        )
                    });
                match builder {
                    Some(builder) => Some(builder?),
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
                .map(|mdir_config| MaildirContextBuilder::new(account_config.clone(), mdir_config)),

            #[cfg(feature = "notmuch")]
            notmuch: toml_account_config
                .notmuch
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Notmuch))
                .map(Clone::clone)
                .map(Arc::new)
                .map(|notmuch_config| {
                    NotmuchContextBuilder::new(account_config.clone(), notmuch_config)
                }),

            #[cfg(feature = "smtp")]
            smtp: toml_account_config
                .smtp
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Smtp))
                .map(Clone::clone)
                .map(Arc::new)
                .map(|smtp_config| SmtpContextBuilder::new(account_config.clone(), smtp_config)),

            #[cfg(feature = "sendmail")]
            sendmail: toml_account_config
                .sendmail
                .as_ref()
                .filter(|_| kinds.contains(&&BackendKind::Sendmail))
                .map(Clone::clone)
                .map(Arc::new)
                .map(|sendmail_config| {
                    SendmailContextBuilder::new(account_config.clone(), sendmail_config)
                }),
        })
    }
}

#[async_trait]
impl email::backend::context::BackendContextBuilder for BackendContextBuilder {
    type Context = BackendContext;

    fn add_folder(&self) -> Option<BackendFeature<Self::Context, dyn AddFolder>> {
        match self.toml_account_config.add_folder_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.add_folder_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.add_folder()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.add_folder_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.add_folder_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn list_folders(&self) -> Option<BackendFeature<Self::Context, dyn ListFolders>> {
        match self.toml_account_config.list_folders_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.list_folders_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.list_folders()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.list_folders_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.list_folders_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn expunge_folder(&self) -> Option<BackendFeature<Self::Context, dyn ExpungeFolder>> {
        match self.toml_account_config.expunge_folder_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.expunge_folder_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.expunge_folder()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.expunge_folder_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.expunge_folder_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn purge_folder(&self) -> Option<BackendFeature<Self::Context, dyn PurgeFolder>> {
        match self.toml_account_config.purge_folder_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.purge_folder_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.purge_folder()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.purge_folder_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.purge_folder_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn delete_folder(&self) -> Option<BackendFeature<Self::Context, dyn DeleteFolder>> {
        match self.toml_account_config.delete_folder_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.delete_folder_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.delete_folder()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.delete_folder_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.delete_folder_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn get_envelope(&self) -> Option<BackendFeature<Self::Context, dyn GetEnvelope>> {
        match self.toml_account_config.get_envelope_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.get_envelope_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.get_envelope()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.get_envelope_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.get_envelope_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn list_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ListEnvelopes>> {
        match self.toml_account_config.list_envelopes_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.list_envelopes_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.list_envelopes()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.list_envelopes_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.list_envelopes_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn watch_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn WatchEnvelopes>> {
        match self.toml_account_config.watch_envelopes_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.watch_envelopes_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.watch_envelopes()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.watch_envelopes_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.watch_envelopes_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn add_flags(&self) -> Option<BackendFeature<Self::Context, dyn AddFlags>> {
        match self.toml_account_config.add_flags_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.add_flags_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.add_flags()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.add_flags_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.add_flags_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn set_flags(&self) -> Option<BackendFeature<Self::Context, dyn SetFlags>> {
        match self.toml_account_config.set_flags_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.set_flags_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.set_flags()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.set_flags_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.set_flags_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn remove_flags(&self) -> Option<BackendFeature<Self::Context, dyn RemoveFlags>> {
        match self.toml_account_config.remove_flags_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.remove_flags_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.remove_flags()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.remove_flags_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.remove_flags_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn add_message(&self) -> Option<BackendFeature<Self::Context, dyn AddMessage>> {
        match self.toml_account_config.add_message_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.add_message_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.add_message()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.add_message_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.add_message_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn send_message(&self) -> Option<BackendFeature<Self::Context, dyn SendMessage>> {
        match self.toml_account_config.send_message_kind() {
            #[cfg(feature = "smtp")]
            Some(BackendKind::Smtp) => self.send_message_with_some(&self.smtp),
            #[cfg(feature = "sendmail")]
            Some(BackendKind::Sendmail) => self.send_message_with_some(&self.sendmail),
            _ => None,
        }
    }

    fn peek_messages(&self) -> Option<BackendFeature<Self::Context, dyn PeekMessages>> {
        match self.toml_account_config.peek_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.peek_messages_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.peek_messages()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.peek_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.peek_messages_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn get_messages(&self) -> Option<BackendFeature<Self::Context, dyn GetMessages>> {
        match self.toml_account_config.get_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.get_messages_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.get_messages()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.get_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.get_messages_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn copy_messages(&self) -> Option<BackendFeature<Self::Context, dyn CopyMessages>> {
        match self.toml_account_config.copy_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.copy_messages_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.copy_messages()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.copy_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.copy_messages_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn move_messages(&self) -> Option<BackendFeature<Self::Context, dyn MoveMessages>> {
        match self.toml_account_config.move_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.move_messages_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.move_messages()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.move_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.move_messages_with_some(&self.notmuch),
            _ => None,
        }
    }

    fn delete_messages(&self) -> Option<BackendFeature<Self::Context, dyn DeleteMessages>> {
        match self.toml_account_config.delete_messages_kind() {
            #[cfg(feature = "imap")]
            Some(BackendKind::Imap) => self.delete_messages_with_some(&self.imap),
            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                let f = self.imap_cache.as_ref()?.delete_messages()?;
                Some(Arc::new(move |ctx| f(ctx.imap_cache.as_ref()?)))
            }
            #[cfg(feature = "maildir")]
            Some(BackendKind::Maildir) => self.delete_messages_with_some(&self.maildir),
            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => self.delete_messages_with_some(&self.notmuch),
            _ => None,
        }
    }

    async fn build(self) -> AnyResult<Self::Context> {
        let mut ctx = BackendContext::default();

        #[cfg(feature = "imap")]
        if let Some(imap) = self.imap {
            ctx.imap = Some(imap.build().await?);
        }

        #[cfg(all(feature = "imap", feature = "account-sync"))]
        if let Some(maildir) = self.imap_cache {
            ctx.imap_cache = Some(maildir.build().await?);
        }

        #[cfg(feature = "maildir")]
        if let Some(maildir) = self.maildir {
            ctx.maildir = Some(maildir.build().await?);
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

#[derive(BackendContext, Default)]
pub struct BackendContext {
    #[cfg(feature = "imap")]
    pub imap: Option<ImapContextSync>,

    #[cfg(all(feature = "imap", feature = "account-sync"))]
    pub imap_cache: Option<MaildirContextSync>,

    #[cfg(feature = "maildir")]
    pub maildir: Option<MaildirContextSync>,

    #[cfg(feature = "notmuch")]
    pub notmuch: Option<NotmuchContextSync>,

    #[cfg(feature = "smtp")]
    pub smtp: Option<SmtpContextSync>,

    #[cfg(feature = "sendmail")]
    pub sendmail: Option<SendmailContextSync>,
}

#[cfg(feature = "imap")]
impl AsRef<Option<ImapContextSync>> for BackendContext {
    fn as_ref(&self) -> &Option<ImapContextSync> {
        &self.imap
    }
}

#[cfg(feature = "maildir")]
impl AsRef<Option<MaildirContextSync>> for BackendContext {
    fn as_ref(&self) -> &Option<MaildirContextSync> {
        &self.maildir
    }
}

#[cfg(feature = "notmuch")]
impl AsRef<Option<NotmuchContextSync>> for BackendContext {
    fn as_ref(&self) -> &Option<NotmuchContextSync> {
        &self.notmuch
    }
}

#[cfg(feature = "smtp")]
impl AsRef<Option<SmtpContextSync>> for BackendContext {
    fn as_ref(&self) -> &Option<SmtpContextSync> {
        &self.smtp
    }
}

#[cfg(feature = "sendmail")]
impl AsRef<Option<SendmailContextSync>> for BackendContext {
    fn as_ref(&self) -> &Option<SendmailContextSync> {
        &self.sendmail
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
                .without_features();

        with_features(&mut backend_builder);

        Ok(Self {
            toml_account_config: toml_account_config.clone(),
            backend: backend_builder.build().await?,
        })
    }

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
                if let Some(_) = &self.toml_account_config.maildir {
                    id_mapper = IdMapper::new(&self.backend.account_config, folder)?;
                }
            }

            #[cfg(all(feature = "imap", feature = "account-sync"))]
            Some(BackendKind::ImapCache) => {
                id_mapper = IdMapper::new(&self.backend.account_config, folder)?;
            }

            #[cfg(feature = "notmuch")]
            Some(BackendKind::Notmuch) => {
                if let Some(_) = &self.toml_account_config.notmuch {
                    id_mapper = IdMapper::new(&self.backend.account_config, folder)?;
                }
            }
            _ => (),
        };

        Ok(id_mapper)
    }

    pub async fn list_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> Result<Envelopes> {
        let backend_kind = self.toml_account_config.list_envelopes_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let envelopes = self.backend.list_envelopes(folder, opts).await?;
        let envelopes =
            Envelopes::from_backend(&self.backend.account_config, &id_mapper, envelopes)?;
        Ok(envelopes)
    }

    pub async fn add_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.add_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flags(folder, &ids, flags).await?;
        Ok(())
    }

    pub async fn add_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.add_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.add_flag(folder, &ids, flag).await?;
        Ok(())
    }

    pub async fn set_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.set_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.set_flags(folder, &ids, flags).await?;
        Ok(())
    }

    pub async fn set_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.set_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.set_flag(folder, &ids, flag).await?;
        Ok(())
    }

    pub async fn remove_flags(&self, folder: &str, ids: &[usize], flags: &Flags) -> Result<()> {
        let backend_kind = self.toml_account_config.remove_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.remove_flags(folder, &ids, flags).await?;
        Ok(())
    }

    pub async fn remove_flag(&self, folder: &str, ids: &[usize], flag: Flag) -> Result<()> {
        let backend_kind = self.toml_account_config.remove_flags_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.remove_flag(folder, &ids, flag).await?;
        Ok(())
    }

    pub async fn add_message(&self, folder: &str, email: &[u8]) -> Result<SingleId> {
        let backend_kind = self.toml_account_config.add_message_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let id = self.backend.add_message(folder, email).await?;
        id_mapper.create_alias(&*id)?;
        Ok(id)
    }

    pub async fn add_message_with_flags(
        &self,
        folder: &str,
        email: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        let backend_kind = self.toml_account_config.add_message_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let id = self
            .backend
            .add_message_with_flags(folder, email, flags)
            .await?;
        id_mapper.create_alias(&*id)?;
        Ok(id)
    }

    pub async fn peek_messages(&self, folder: &str, ids: &[usize]) -> Result<Messages> {
        let backend_kind = self.toml_account_config.get_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        let msgs = self.backend.peek_messages(folder, &ids).await?;
        Ok(msgs)
    }

    pub async fn get_messages(&self, folder: &str, ids: &[usize]) -> Result<Messages> {
        let backend_kind = self.toml_account_config.get_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        let msgs = self.backend.get_messages(folder, &ids).await?;
        Ok(msgs)
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
            .await?;
        Ok(())
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
            .await?;
        Ok(())
    }

    pub async fn delete_messages(&self, folder: &str, ids: &[usize]) -> Result<()> {
        let backend_kind = self.toml_account_config.delete_messages_kind();
        let id_mapper = self.build_id_mapper(folder, backend_kind)?;
        let ids = Id::multiple(id_mapper.get_ids(ids)?);
        self.backend.delete_messages(folder, &ids).await?;
        Ok(())
    }

    pub async fn send_message_then_save_copy(&self, msg: &[u8]) -> Result<()> {
        self.backend.send_message_then_save_copy(msg).await?;
        Ok(())
    }

    pub async fn watch_envelopes(&self, folder: &str) -> Result<()> {
        self.backend.watch_envelopes(folder).await?;
        Ok(())
    }
}

impl Deref for Backend {
    type Target = email::backend::Backend<BackendContext>;

    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}
