use anyhow::Result;
use dialoguer::Input;
use pimalaya_email::{BackendConfig, NotmuchBackend, NotmuchConfig};

use super::THEME;

pub(crate) fn configure() -> Result<BackendConfig> {
    let mut notmuch_config = NotmuchConfig::default();

    notmuch_config.db_path = match NotmuchBackend::get_default_db_path() {
        Ok(db) => db,
        _ => {
            let input: String = Input::with_theme(&*THEME)
                .with_prompt("Could not find a notmuch database. Enter path manually:")
                .interact_text()?;
            input.into()
        }
    };

    Ok(BackendConfig::Notmuch(notmuch_config))
}
