use color_eyre::Result;
use email::sendmail::config::SendmailConfig;
use inquire::Text;

use crate::backend::config::BackendConfig;

pub(crate) fn configure() -> Result<BackendConfig> {
    let config = SendmailConfig {
        cmd: Text::new("Sendmail-compatible shell command to send emails")
            .with_default("/usr/bin/msmtp")
            .prompt()?
            .into(),
    };

    Ok(BackendConfig::Sendmail(config))
}
