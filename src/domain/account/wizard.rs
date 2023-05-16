use anyhow::{anyhow, Result};
use dialoguer::Input;
use email_address::EmailAddress;

use crate::{backend, config::wizard::THEME, sender};

use super::DeserializedAccountConfig;

pub(crate) fn configure() -> Result<Option<(String, DeserializedAccountConfig)>> {
    let mut config = DeserializedAccountConfig::default();

    let account_name = Input::with_theme(&*THEME)
        .with_prompt("Account name")
        .default(String::from("Personal"))
        .interact()?;

    config.email = Input::with_theme(&*THEME)
        .with_prompt("Email address")
        .validate_with(|email: &String| {
            if EmailAddress::is_valid(email) {
                Ok(())
            } else {
                Err(anyhow!("Invalid email address: {email}"))
            }
        })
        .interact()?;

    config.display_name = Some(
        Input::with_theme(&*THEME)
            .with_prompt("Full display name")
            .interact()?,
    );

    config.backend = backend::wizard::configure(&account_name, &config.email)?;

    config.sender = sender::wizard::configure(&account_name, &config.email)?;

    Ok(Some((account_name, config)))
}
