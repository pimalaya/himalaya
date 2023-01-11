use super::THEME;
use anyhow::Result;
use dialoguer::Input;
use himalaya_lib::{EmailSender, SendmailConfig};

pub(crate) fn configure() -> Result<EmailSender> {
    Ok(EmailSender::Sendmail(SendmailConfig {
        cmd: Input::with_theme(&*THEME)
            .with_prompt("Enter an external command to send a mail: ")
            .default("/usr/bin/msmtp".to_owned())
            .interact()?,
    }))
}
