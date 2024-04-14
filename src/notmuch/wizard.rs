use color_eyre::Result;
use dialoguer::Input;
use email::notmuch::config::NotmuchConfig;

use crate::{backend::config::BackendConfig, ui::THEME};

pub(crate) fn configure() -> Result<BackendConfig> {
    let mut config = NotmuchConfig::default();

    let default_database_path = NotmuchConfig::get_default_database_path()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    config.database_path = Some(
        Input::with_theme(&*THEME)
            .with_prompt("Notmuch database path")
            .default(default_database_path)
            .interact_text()?
            .into(),
    );

    Ok(BackendConfig::Notmuch(config))
}
