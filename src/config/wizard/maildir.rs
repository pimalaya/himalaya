use super::THEME;
use crate::account::{
    DeserializedAccountConfig, DeserializedBaseAccountConfig, DeserializedMaildirAccountConfig,
};
use anyhow::Result;
use dialoguer::Input;
use dirs::home_dir;
use himalaya_lib::MaildirConfig;

#[cfg(feature = "maildir-backend")]
pub(crate) fn configure(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
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

    Ok(DeserializedAccountConfig::Maildir(
        DeserializedMaildirAccountConfig {
            base,
            backend: MaildirConfig {
                root_dir: input.into(),
            },
        },
    ))
}
