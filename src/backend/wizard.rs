use color_eyre::Result;
use dialoguer::Select;
#[cfg(feature = "account-discovery")]
use email::account::discover::config::AutoConfig;

#[cfg(feature = "imap")]
use crate::imap;
#[cfg(feature = "maildir")]
use crate::maildir;
#[cfg(feature = "notmuch")]
use crate::notmuch;
#[cfg(feature = "sendmail")]
use crate::sendmail;
#[cfg(feature = "smtp")]
use crate::smtp;
use crate::ui::THEME;

use super::{config::BackendConfig, BackendKind};

const DEFAULT_BACKEND_KINDS: &[BackendKind] = &[
    #[cfg(feature = "imap")]
    BackendKind::Imap,
    #[cfg(feature = "maildir")]
    BackendKind::Maildir,
    #[cfg(feature = "notmuch")]
    BackendKind::Notmuch,
];

const SEND_MESSAGE_BACKEND_KINDS: &[BackendKind] = &[
    #[cfg(feature = "smtp")]
    BackendKind::Smtp,
    #[cfg(feature = "sendmail")]
    BackendKind::Sendmail,
];

pub(crate) async fn configure(
    account_name: &str,
    email: &str,
    #[cfg(feature = "account-discovery")] autoconfig: Option<&AutoConfig>,
) -> Result<Option<BackendConfig>> {
    let kind = Select::with_theme(&*THEME)
        .with_prompt("Default email backend")
        .items(DEFAULT_BACKEND_KINDS)
        .default(0)
        .interact_opt()?
        .and_then(|idx| DEFAULT_BACKEND_KINDS.get(idx).map(Clone::clone));

    let config = match kind {
        #[cfg(feature = "imap")]
        Some(kind) if kind == BackendKind::Imap => Some(
            imap::wizard::configure(
                account_name,
                email,
                #[cfg(feature = "account-discovery")]
                autoconfig,
            )
            .await?,
        ),
        #[cfg(feature = "maildir")]
        Some(kind) if kind == BackendKind::Maildir => Some(maildir::wizard::configure()?),
        #[cfg(feature = "notmuch")]
        Some(kind) if kind == BackendKind::Notmuch => Some(notmuch::wizard::configure()?),
        _ => None,
    };

    Ok(config)
}

pub(crate) async fn configure_sender(
    account_name: &str,
    email: &str,
    #[cfg(feature = "account-discovery")] autoconfig: Option<&AutoConfig>,
) -> Result<Option<BackendConfig>> {
    let kind = Select::with_theme(&*THEME)
        .with_prompt("Backend for sending messages")
        .items(SEND_MESSAGE_BACKEND_KINDS)
        .default(0)
        .interact_opt()?
        .and_then(|idx| SEND_MESSAGE_BACKEND_KINDS.get(idx).map(Clone::clone));

    let config = match kind {
        #[cfg(feature = "smtp")]
        Some(kind) if kind == BackendKind::Smtp => Some(
            smtp::wizard::configure(
                account_name,
                email,
                #[cfg(feature = "account-discovery")]
                autoconfig,
            )
            .await?,
        ),
        #[cfg(feature = "sendmail")]
        Some(kind) if kind == BackendKind::Sendmail => Some(sendmail::wizard::configure()?),
        _ => None,
    };

    Ok(config)
}
