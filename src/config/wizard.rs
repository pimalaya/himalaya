use anyhow::Result;
use dialoguer::{Confirm, theme::ColorfulTheme};
use log::trace;
use std::path::PathBuf;

use crate::config::DeserializedConfig;

pub(crate) fn wizard() -> Result<PathBuf> {
    trace!(">> wizard");
    println!("Himalaya couldn't find an already existing configuration file.\n");

    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to create one in the wizard?")
        .default(true)
        .interact_opt()?;

    match confirm {
        Some(false) | None => std::process::exit(0),
        _ => {},
    }

    let _config = DeserializedConfig::default();

    // Populate config with user input

    // Determine path to save to

    // Serialize config to file

    trace!("<< wizard");
    Ok(PathBuf::new())
}
