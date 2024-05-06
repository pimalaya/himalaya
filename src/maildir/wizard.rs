use color_eyre::Result;
use dirs::home_dir;
use email::maildir::config::MaildirConfig;
use inquire::Text;

use crate::backend::config::BackendConfig;

pub(crate) fn configure() -> Result<BackendConfig> {
    let mut config = MaildirConfig::default();

    let mut input = Text::new("Maildir directory");

    let Some(home) = home_dir() else {
        config.root_dir = input.prompt()?.into();

        return Ok(BackendConfig::Maildir(config));
    };

    let def = home.join("Mail").display().to_string();
    input = input.with_default(&def);

    config.root_dir = input.prompt()?.into();

    Ok(BackendConfig::Maildir(config))
}
