use anyhow::Result;
use dialoguer::Input;
use email::sender::{SenderConfig, SendmailConfig};

use crate::config::wizard::THEME;

pub(crate) fn configure() -> Result<SenderConfig> {
    let mut config = SendmailConfig::default();

    config.cmd = Input::with_theme(&*THEME)
        .with_prompt("Sendmail-compatible shell command to send emails")
        .default(String::from("/usr/bin/msmtp"))
        .interact()?
        .into();

    Ok(SenderConfig::Sendmail(config))
}
