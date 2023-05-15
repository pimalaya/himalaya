#[cfg(feature = "imap-backend")]
pub(crate) mod imap;
mod maildir;
#[cfg(feature = "notmuch-backend")]
mod notmuch;
mod sendmail;
mod smtp;
mod validators;

use super::DeserializedConfig;
use crate::account::DeserializedAccountConfig;
use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use log::trace;
use once_cell::sync::Lazy;
use pimalaya_email::{BackendConfig, SenderConfig};
use std::{fs, io, process};

const BACKENDS: &[&str] = &[
    #[cfg(feature = "imap-backend")]
    "IMAP",
    "Maildir",
    #[cfg(feature = "notmuch-backend")]
    "Notmuch",
    "None",
];

const SENDERS: &[&str] = &["SMTP", "Sendmail"];

const SECURITY_PROTOCOLS: &[&str] = &["SSL/TLS", "STARTTLS", "None"];

const AUTH_MECHANISMS: &[&str] = &[PASSWD, OAUTH2];
const PASSWD: &str = "Password";
const OAUTH2: &str = "OAuth 2.0";

const SECRET: &[&str] = &[RAW, CMD, KEYRING];
const RAW: &str = "In clear, in your configuration (not recommanded)";
const CMD: &str = "From a shell command";
const KEYRING: &str = "From your system's global keyring";

// A wizard should have pretty colors 💅
static THEME: Lazy<ColorfulTheme> = Lazy::new(ColorfulTheme::default);

pub(crate) fn wizard() -> Result<DeserializedConfig> {
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
    // let path = dirs::config_dir()
    //     .map(|p| p.join("himalaya").join("config.toml"))
    //     .ok_or_else(|| anyhow!("The wizard could not determine the config directory. Aborting"))?;
    let path = std::path::PathBuf::from("/home/soywod/config.wizard.toml");

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
    let default_account = match config.accounts.len() {
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

    if let Some(account) = default_account {
        account.default = Some(true);
    }

    // Serialize config to file
    println!("\nWriting the configuration to {path:?}...");
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, toml::to_string(&config)?)?;

    trace!("<< wizard");
    Ok(config)
}

fn configure_account() -> Result<Option<DeserializedAccountConfig>> {
    let mut config = DeserializedAccountConfig::default();

    config.email = Input::with_theme(&*THEME)
        .with_prompt("What is your email address?")
        .validate_with(validators::EmailValidator)
        .interact()?;

    config.display_name = Some(
        Input::with_theme(&*THEME)
            .with_prompt("Which name would you like to display with your email?")
            .interact()?,
    );

    let backend = Select::with_theme(&*THEME)
        .with_prompt("Which backend would you like to configure your account for?")
        .items(BACKENDS)
        .default(0)
        .interact_opt()?;

    config.backend = match backend {
        Some(idx) if BACKENDS[idx] == "IMAP" => imap::configure(&config),
        Some(idx) if BACKENDS[idx] == "Maildir" => maildir::configure(),
        Some(idx) if BACKENDS[idx] == "Notmuch" => notmuch::configure(),
        Some(idx) if BACKENDS[idx] == "None" => Ok(BackendConfig::None),
        _ => return Ok(None),
    }?;

    let sender = Select::with_theme(&*THEME)
        .with_prompt("Which sender would you like use with your account?")
        .items(SENDERS)
        .default(0)
        .interact_opt()?;

    config.sender = match sender {
        Some(idx) if SENDERS[idx] == "SMTP" => smtp::configure(&config),
        Some(idx) if SENDERS[idx] == "Sendmail" => sendmail::configure(),
        Some(idx) if SENDERS[idx] == "None" => Ok(SenderConfig::None),
        _ => return Ok(None),
    }?;

    Ok(Some(config))
}

pub(crate) fn prompt_passwd(prompt: &str) -> io::Result<String> {
    Password::with_theme(&*THEME)
        .with_prompt(prompt)
        .with_confirmation(
            "Confirm password:",
            "Passwords do not match, please try again.",
        )
        .interact()
}

pub(crate) fn prompt_secret(prompt: &str) -> io::Result<String> {
    Input::with_theme(&*THEME)
        .with_prompt(prompt)
        .report(false)
        .interact()
}
