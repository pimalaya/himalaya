#[cfg(feature = "imap-backend")]
mod imap;
mod maildir;
#[cfg(feature = "notmuch-backend")]
mod notmuch;
mod sendmail;
mod smtp;
mod validators;

use super::DeserializedConfig;
use crate::account::{DeserializedAccountConfig, DeserializedBaseAccountConfig};
use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use log::trace;
use once_cell::sync::Lazy;
use std::{fs, process};

const BACKENDS: &[&str] = &[
    "Maildir",
    #[cfg(feature = "imap-backend")]
    "IMAP",
    #[cfg(feature = "notmuch-backend")]
    "Notmuch",
];

const SENDERS: &[&str] = &["SMTP", "Sendmail"];

const SECURITY_PROTOCOLS: &[&str] = &["SSL/TLS", "STARTTLS", "None"];

// A wizard should have pretty colors 💅
static THEME: Lazy<ColorfulTheme> = Lazy::new(ColorfulTheme::default);

pub(crate) fn wizard() -> Result<DeserializedConfig> {
    trace!(">> wizard");
    println!("Himalaya couldn't find an already existing configuration file.");

    match Confirm::new()
        .with_prompt("Do you want to create one with the wizard?")
        .default(true)
        .report(false)
        .interact_opt()?
    {
        Some(false) | None => process::exit(0),
        _ => {}
    }

    // Determine path to save to
    let path = dirs::config_dir()
        .map(|p| p.join("himalaya").join("config.toml"))
        .ok_or_else(|| anyhow!("The wizard could not determine the config directory. Aborting"))?;

    let mut config = DeserializedConfig::default();

    // Setup one or multiple accounts
    println!("\n{}", style("First let's setup an account").underlined());
    while let Some(account_config) = configure_account()? {
        let name: String = Input::with_theme(&*THEME)
            .with_prompt("What would you like to name your account?")
            .default("Personal".to_owned())
            .interact()?;

        config.accounts.insert(name, account_config);

        match Confirm::new()
            .with_prompt("Setup another account?")
            .default(false)
            .report(false)
            .interact_opt()?
        {
            Some(true) => println!("\n{}", style("Setting up another account").underlined()),
            _ => break,
        }
    }

    // If one acounts is setup, make it the default. If multiple accounts are setup, decide which
    // will be the default. If no accounts are setup, exit the process
    let default = match config.accounts.len() {
        1 => Some(config.accounts.values_mut().next().unwrap()),
        i if i > 1 => {
            let accounts = config.accounts.clone();
            let accounts: Vec<&String> = accounts.keys().collect();

            println!(
                "\n{}",
                style(format!("You've setup {} accounts", accounts.len())).underlined()
            );
            match Select::with_theme(&*THEME)
                .with_prompt("Which account would you like to set as your default?")
                .items(&accounts)
                .default(0)
                .interact_opt()?
            {
                Some(i) => Some(config.accounts.get_mut(accounts[i]).unwrap()),
                _ => process::exit(0),
            }
        }
        _ => process::exit(0),
    };

    match default {
        Some(DeserializedAccountConfig::None(default)) => default.default = Some(true),
        Some(DeserializedAccountConfig::Maildir(default)) => default.base.default = Some(true),
        #[cfg(feature = "imap-backend")]
        Some(DeserializedAccountConfig::Imap(default)) => default.base.default = Some(true),
        #[cfg(feature = "notmuch-backend")]
        Some(DeserializedAccountConfig::Notmuch(default)) => default.base.default = Some(true),
        _ => {}
    }

    // Serialize config to file
    println!("\nWriting the configuration to {path:?}...");
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, toml::to_vec(&config)?)?;

    trace!("<< wizard");
    Ok(config)
}

fn configure_account() -> Result<Option<DeserializedAccountConfig>> {
    let mut base = configure_base()?;
    let sender = Select::with_theme(&*THEME)
        .with_prompt("Which sender would you like use with your account?")
        .items(SENDERS)
        .default(0)
        .interact_opt()?;

    base.email_sender = match sender {
        Some(idx) if SENDERS[idx] == "SMTP" => smtp::configure(&base),
        Some(idx) if SENDERS[idx] == "Sendmail" => sendmail::configure(),
        _ => return Ok(None),
    }?;

    let backend = Select::with_theme(&*THEME)
        .with_prompt("Which backend would you like to configure your account for?")
        .items(BACKENDS)
        .default(0)
        .interact_opt()?;

    match backend {
        Some(idx) if BACKENDS[idx] == "Maildir" => Ok(Some(maildir::configure(base)?)),
        #[cfg(feature = "imap-backend")]
        Some(idx) if BACKENDS[idx] == "IMAP" => Ok(Some(imap::configure(base)?)),
        #[cfg(feature = "notmuch-backend")]
        Some(idx) if BACKENDS[idx] == "Notmuch" => Ok(Some(notmuch::configure(base)?)),
        _ => Ok(None),
    }
}

fn configure_base() -> Result<DeserializedBaseAccountConfig> {
    let mut base_account_config = DeserializedBaseAccountConfig {
        email: Input::with_theme(&*THEME)
            .with_prompt("Enter your email:")
            .validate_with(validators::EmailValidator)
            .interact()?,
        ..Default::default()
    };

    base_account_config.display_name = Some(
        Input::with_theme(&*THEME)
            .with_prompt("Enter display name:")
            .interact()?,
    );

    Ok(base_account_config)
}
