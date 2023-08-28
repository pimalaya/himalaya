use anyhow::Result;
use dialoguer::Input;
use email::backend::{BackendConfig, NotmuchBackend, NotmuchConfig};

use crate::config::wizard::THEME;

pub(crate) fn configure() -> Result<BackendConfig> {
    let mut config = NotmuchConfig::default();

    config.db_path = if let Ok(db_path) = NotmuchBackend::get_default_db_path() {
        db_path
    } else {
        let db_path: String = Input::with_theme(&*THEME)
            .with_prompt("Notmuch database path")
            .interact_text()?;
        db_path.into()
    };

    Ok(BackendConfig::Notmuch(config))
}
