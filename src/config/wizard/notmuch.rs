use super::THEME;
use crate::account::{
    DeserializedAccountConfig, DeserializedBaseAccountConfig, DeserializedNotmuchAccountConfig,
};
use anyhow::Result;
use dialoguer::Input;
use himalaya_lib::{NotmuchBackend, NotmuchConfig};

pub(crate) fn configure(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
    let db_path = match NotmuchBackend::get_default_db_path() {
        Ok(db) => db,
        _ => {
            let input: String = Input::with_theme(&*THEME)
                .with_prompt("Could not find a notmuch database. Enter path manually:")
                .interact_text()?;
            input.into()
        }
    };

    let backend = NotmuchConfig { db_path };

    Ok(DeserializedAccountConfig::Notmuch(
        DeserializedNotmuchAccountConfig { base, backend },
    ))
}
