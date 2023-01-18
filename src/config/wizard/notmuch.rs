use super::THEME;
use crate::account::{
    DeserializedAccountConfig, DeserializedBaseAccountConfig, DeserializedNotmuchAccountConfig,
};
use anyhow::Result;
use dialoguer::Input;
use himalaya_lib::NotmuchConfig;
use std::path::PathBuf;

#[cfg(feature = "notmuch-backend")]
pub(crate) fn configure(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {

    let db_path: PathBuf = match std::process::Command::new("notmuch")
        .args(["config", "get", "database.path"])
        .output()
    {
        Ok(output) => PathBuf::from(String::from_utf8(output.stdout)?),
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
