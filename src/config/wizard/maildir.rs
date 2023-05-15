use anyhow::Result;
use dialoguer::Input;
use dirs::home_dir;
use pimalaya_email::{BackendConfig, MaildirConfig};

use super::THEME;

pub(crate) fn configure() -> Result<BackendConfig> {
    let mut maildir_config = MaildirConfig::default();

    let input = if let Some(home) = home_dir() {
        Input::with_theme(&*THEME)
            .default(home.join("Mail").display().to_string())
            .with_prompt("Enter the path to your maildir")
            .interact_text()?
    } else {
        Input::with_theme(&*THEME)
            .with_prompt("Enter the path to your maildir")
            .interact_text()?
    };

    maildir_config.root_dir = input.into();

    Ok(BackendConfig::Maildir(maildir_config))
}
