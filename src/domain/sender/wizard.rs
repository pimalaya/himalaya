use anyhow::Result;
use dialoguer::Select;
use pimalaya_email::sender::SenderConfig;

use crate::config::wizard::THEME;

use super::sendmail;
#[cfg(feature = "smtp-sender")]
use super::smtp;

#[cfg(feature = "smtp-sender")]
const SMTP: &str = "SMTP";
const SENDMAIL: &str = "Sendmail";
const NONE: &str = "None";

const SENDERS: &[&str] = &[
    #[cfg(feature = "smtp-sender")]
    SMTP,
    SENDMAIL,
    NONE,
];

pub(crate) async fn configure(account_name: &str, email: &str) -> Result<SenderConfig> {
    let sender = Select::with_theme(&*THEME)
        .with_prompt("Email sender")
        .items(SENDERS)
        .default(0)
        .interact_opt()?;

    match sender {
        #[cfg(feature = "smtp-sender")]
        Some(n) if SENDERS[n] == SMTP => smtp::wizard::configure(account_name, email).await,
        Some(n) if SENDERS[n] == SENDMAIL => sendmail::wizard::configure(),
        _ => Ok(SenderConfig::None),
    }
}
