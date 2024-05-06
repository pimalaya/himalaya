use color_eyre::Result;
use email::notmuch::config::NotmuchConfig;
use inquire::Text;

use crate::backend::config::BackendConfig;

pub(crate) fn configure() -> Result<BackendConfig> {
    let config = NotmuchConfig {
        database_path: Some(
            Text::new("Notmuch database path")
                .with_default(
                    &NotmuchConfig::get_default_database_path()
                        .unwrap_or_default()
                        .to_string_lossy(),
                )
                .prompt()?
                .into(),
        ),
        ..Default::default()
    };

    Ok(BackendConfig::Notmuch(config))
}
