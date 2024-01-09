use anyhow::Result;
use dialoguer::Input;
use email::sendmail::config::SendmailConfig;

use crate::{backend::config::BackendConfig, ui::THEME};

pub(crate) fn configure() -> Result<BackendConfig> {
    let config = SendmailConfig {
        cmd: Input::with_theme(&*THEME)
            .with_prompt("Sendmail-compatible shell command to send emails")
            .default(String::from("/usr/bin/msmtp"))
            .interact()?
            .into(),
        // ..Default::default() // in case any other field was added
    };

    Ok(BackendConfig::Sendmail(config))
}
