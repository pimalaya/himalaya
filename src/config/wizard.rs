use super::DeserializedConfig;
use crate::account;
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use once_cell::sync::Lazy;
use shellexpand_utils::{shellexpand_path, try_shellexpand_path};
use std::{env, fs, io, process};

#[macro_export]
macro_rules! wizard_warn {
    ($($arg:tt)*) => {
	println!("{}", console::style(format!($($arg)*)).yellow().bold());
    };
}

#[macro_export]
macro_rules! wizard_prompt {
    ($($arg:tt)*) => {
	format!("{}", console::style(format!($($arg)*)).italic())
    };
}

#[macro_export]
macro_rules! wizard_log {
    ($($arg:tt)*) => {
	println!("");
	println!("{}", console::style(format!($($arg)*)).underlined());
	println!("");
    };
}

pub(crate) static THEME: Lazy<ColorfulTheme> = Lazy::new(ColorfulTheme::default);

pub(crate) async fn configure() -> Result<DeserializedConfig> {
    wizard_log!("Configuring your first account:");

    let mut config = DeserializedConfig::default();

    while let Some((name, account_config)) = account::wizard::configure().await? {
        config.accounts.insert(name, account_config);

        if !Confirm::new()
            .with_prompt(wizard_prompt!(
                "Would you like to configure another account?"
            ))
            .default(false)
            .interact_opt()?
            .unwrap_or_default()
        {
            break;
        }

        wizard_log!("Configuring another account:");
    }

    // If one account is setup, make it the default. If multiple
    // accounts are setup, decide which will be the default. If no
    // accounts are setup, exit the process.
    let default_account = match config.accounts.len() {
        0 => process::exit(0),
        1 => Some(config.accounts.values_mut().next().unwrap()),
        _ => {
            let accounts = config.accounts.clone();
            let accounts: Vec<&String> = accounts.keys().collect();

            println!("{} accounts have been configured.", accounts.len());

            Select::with_theme(&*THEME)
                .with_prompt(wizard_prompt!(
                    "Which account would you like to set as your default?"
                ))
                .items(&accounts)
                .default(0)
                .interact_opt()?
                .and_then(|idx| config.accounts.get_mut(accounts[idx]))
        }
    };

    if let Some(account) = default_account {
        account.default = Some(true);
    } else {
        process::exit(0)
    }

    let path = Input::with_theme(&*THEME)
        .with_prompt(wizard_prompt!(
            "Where would you like to save your configuration?"
        ))
        .default(
            dirs::config_dir()
                .map(|p| p.join("himalaya").join("config.toml"))
                .unwrap_or_else(|| env::temp_dir().join("himalaya").join("config.toml"))
                .to_string_lossy()
                .to_string(),
        )
        .validate_with(|path: &String| try_shellexpand_path(path).map(|_| ()))
        .interact()?;
    let path = shellexpand_path(&path);

    println!("Writing the configuration to {path:?}…");

    fs::create_dir_all(path.parent().unwrap_or(&path))?;
    fs::write(path, toml::to_string(&config)?)?;

    Ok(config)
}

pub(crate) fn prompt_passwd(prompt: &str) -> io::Result<String> {
    Password::with_theme(&*THEME)
        .with_prompt(prompt)
        .with_confirmation(
            "Confirm password",
            "Passwords do not match, please try again.",
        )
        .interact()
}

pub(crate) fn prompt_secret(prompt: &str) -> io::Result<String> {
    Password::with_theme(&*THEME)
        .with_prompt(prompt)
        .report(false)
        .interact()
}
