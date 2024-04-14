use color_eyre::Result;
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
    };

    Ok(BackendConfig::Sendmail(config))
}
