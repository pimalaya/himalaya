use anyhow::Result;
use dialoguer::Input;
use pimalaya_email::{SenderConfig, SendmailConfig};

use super::THEME;

pub(crate) fn configure() -> Result<SenderConfig> {
    let mut sendmail_config = SendmailConfig::default();

    sendmail_config.cmd = Input::with_theme(&*THEME)
        .with_prompt("Enter an external command to send an email: ")
        .default("/usr/bin/msmtp".to_owned())
        .interact()?
        .into();

    Ok(SenderConfig::Sendmail(sendmail_config))
}
