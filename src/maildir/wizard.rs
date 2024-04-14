use color_eyre::Result;
use dialoguer::Input;
use dirs::home_dir;
use email::maildir::config::MaildirConfig;

use crate::{backend::config::BackendConfig, ui::THEME};

pub(crate) fn configure() -> Result<BackendConfig> {
    let mut config = MaildirConfig::default();

    let mut input = Input::with_theme(&*THEME);

    if let Some(home) = home_dir() {
        input.default(home.join("Mail").display().to_string());
    };

    config.root_dir = input
        .with_prompt("Maildir directory")
        .interact_text()?
        .into();

    Ok(BackendConfig::Maildir(config))
}
