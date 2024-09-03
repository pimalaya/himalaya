use color_eyre::Result;
use email::autoconfig::config::AutoConfig;
use email_address::EmailAddress;
use pimalaya_tui::{prompt, wizard};

use super::{config::BackendConfig, BackendKind};

const DEFAULT_BACKEND_KINDS: &[BackendKind] = &[
    #[cfg(feature = "imap")]
    BackendKind::Imap,
    #[cfg(feature = "maildir")]
    BackendKind::Maildir,
    #[cfg(feature = "notmuch")]
    BackendKind::Notmuch,
];

pub async fn configure(
    account_name: &str,
    email: &EmailAddress,
    autoconfig: Option<&AutoConfig>,
) -> Result<BackendConfig> {
    let backend = prompt::item("Default backend:", &*DEFAULT_BACKEND_KINDS, None)?;

    match backend {
        #[cfg(feature = "imap")]
        BackendKind::Imap => {
            let config = wizard::imap::start(account_name, email, autoconfig).await?;
            Ok(BackendConfig::Imap(config))
        }
        #[cfg(feature = "maildir")]
        BackendKind::Maildir => {
            let config = wizard::maildir::start(account_name)?;
            Ok(BackendConfig::Maildir(config))
        }
        #[cfg(feature = "notmuch")]
        BackendKind::Notmuch => {
            let config = wizard::notmuch::start()?;
            Ok(BackendConfig::Notmuch(config))
        }
        _ => unreachable!(),
    }
}

const SEND_MESSAGE_BACKEND_KINDS: &[BackendKind] = &[
    #[cfg(feature = "smtp")]
    BackendKind::Smtp,
    #[cfg(feature = "sendmail")]
    BackendKind::Sendmail,
];

pub async fn configure_sender(
    account_name: &str,
    email: &EmailAddress,
    autoconfig: Option<&AutoConfig>,
) -> Result<BackendConfig> {
    let backend = prompt::item(
        "Backend for sending messages:",
        &*SEND_MESSAGE_BACKEND_KINDS,
        None,
    )?;

    match backend {
        #[cfg(feature = "smtp")]
        BackendKind::Smtp => {
            let config = wizard::smtp::start(account_name, email, autoconfig).await?;
            Ok(BackendConfig::Smtp(config))
        }
        #[cfg(feature = "sendmail")]
        BackendKind::Sendmail => {
            let config = wizard::sendmail::start()?;
            Ok(BackendConfig::Sendmail(config))
        }
        _ => unreachable!(),
    }
}
