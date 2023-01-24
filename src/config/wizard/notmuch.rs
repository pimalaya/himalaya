use super::THEME;
use crate::account::{
    DeserializedAccountConfig, DeserializedBaseAccountConfig, DeserializedNotmuchAccountConfig,
};
use anyhow::Result;
use dialoguer::Input;
use himalaya_lib::{
    notmuch::{Database, DatabaseMode},
    NotmuchConfig,
};
use std::path::PathBuf;

pub(crate) fn configure(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
    let db_path = match Database::open_with_config(None, DatabaseMode::ReadOnly, None, None) {
        Ok(db) => db.path(),
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
