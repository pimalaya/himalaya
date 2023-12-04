use anyhow::Result;
use dialoguer::Input;
use email::sendmail::config::SendmailConfig;

use crate::{backend::config::BackendConfig, config::wizard::THEME};

pub(crate) fn configure() -> Result<BackendConfig> {
    let mut config = SendmailConfig::default();

    config.cmd = Input::with_theme(&*THEME)
        .with_prompt("Sendmail-compatible shell command to send emails")
        .default(String::from("/usr/bin/msmtp"))
        .interact()?
        .into();

    Ok(BackendConfig::Sendmail(config))
}
