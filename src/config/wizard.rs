use super::DeserializedConfig;
use crate::account::{DeserializedAccountConfig, DeserializedBaseAccountConfig};
use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select, Validator};
use email_address::EmailAddress;
use himalaya_lib::{EmailSender, SendmailConfig, SmtpConfig};
use log::trace;
use once_cell::sync::Lazy;

const BACKENDS: &[&str] = &[
    #[cfg(feature = "imap-backend")]
    "IMAP",
    #[cfg(feature = "maildir-backend")]
    "Maildir",
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
        Some(false) | None => std::process::exit(0),
        _ => {}
    }

    // Determine path to save to
    let path = dirs::config_dir()
        .map(|p| p.join("himalaya").join("config.toml"))
        .ok_or_else(|| anyhow!("The wizard could not determine the config directory. Aborting"))?;

    let mut config = DeserializedConfig::default();

    // Setup one or multiple accounts
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
            Some(true) => {}
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
            match Select::with_theme(&*THEME)
                .with_prompt("Which account would you like to set as your default?")
                .items(&accounts)
                .default(0)
                .interact_opt()?
            {
                Some(i) => Some(config.accounts.get_mut(accounts[i]).unwrap()),
                _ => std::process::exit(0),
            }
        }
        _ => std::process::exit(0),
    };

    match default {
        Some(DeserializedAccountConfig::None(default)) => default.default = Some(true),
        #[cfg(feature = "imap-backend")]
        Some(DeserializedAccountConfig::Imap(default)) => default.base.default = Some(true),
        #[cfg(feature = "maildir-backend")]
        Some(DeserializedAccountConfig::Maildir(default)) => default.base.default = Some(true),
        #[cfg(feature = "notmuch-backend")]
        Some(DeserializedAccountConfig::Notmuch(default)) => default.base.default = Some(true),
        _ => {}
    }

    // Serialize config to file
    println!("Writing the configuration to {:?}", path);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, toml::to_vec(&config)?)?;

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
        Some(idx) if SENDERS[idx] == "SMTP" => configure_smtp(&base),
        Some(idx) if SENDERS[idx] == "Sendmail" => configure_sendmail(),
        _ => return Ok(None),
    }?;

    let backend = Select::with_theme(&*THEME)
        .with_prompt("Which backend would you like to configure your account for?")
        .items(BACKENDS)
        .default(0)
        .interact_opt()?;

    match backend {
        #[cfg(feature = "imap-backend")]
        Some(idx) if BACKENDS[idx] == "IMAP" => Ok(Some(configure_imap(base)?)),
        #[cfg(feature = "maildir-backend")]
        Some(idx) if BACKENDS[idx] == "Maildir" => Ok(Some(configure_maildir(base)?)),
        #[cfg(feature = "notmuch-backend")]
        Some(idx) if BACKENDS[idx] == "Notmuch" => Ok(Some(configure_notmuch(base)?)),
        _ => Ok(None),
    }
}

fn configure_smtp(base: &DeserializedBaseAccountConfig) -> Result<EmailSender> {
    let mut smtp_config = SmtpConfig::default();

    smtp_config.host = Input::with_theme(&*THEME)
        .with_prompt("Enter the SMTP host: ")
        .default(format!("smtp.{}", base.email.rsplit_once('@').unwrap().1))
        .interact()?;

    smtp_config.port = Input::with_theme(&*THEME)
        .with_prompt("Enter the SMTP port:")
        .validate_with(|input: &String| input.parse::<u16>().map(|_| ()))
        .default(465.to_string())
        .interact()
        .map(|input| input.parse::<u16>().unwrap())?;

    smtp_config.login = Input::with_theme(&*THEME)
        .with_prompt("Enter your SMTP login:")
        .default(base.email.clone())
        .interact()?;

    smtp_config.passwd_cmd = Input::with_theme(&*THEME)
        .with_prompt("What shell command should we run to get your password?")
        .default(format!("pass show {}", &base.email))
        .interact()?;

    match Select::with_theme(&*THEME)
        .with_prompt("Which security protocol do you want to use?")
        .items(SECURITY_PROTOCOLS)
        .default(0)
        .interact_opt()?
    {
        Some(idx) if SECURITY_PROTOCOLS[idx] == "SSL/TLS" => smtp_config.ssl = Some(true),
        Some(idx) if SECURITY_PROTOCOLS[idx] == "STARTTLS" => smtp_config.starttls = Some(true),
        _ => {}
    };

    Ok(EmailSender::Smtp(smtp_config))
}

fn configure_sendmail() -> Result<EmailSender> {
    Ok(EmailSender::Sendmail(SendmailConfig {
        cmd: Input::with_theme(&*THEME)
            .with_prompt("Enter an external command to send a mail: ")
            .default("/usr/bin/msmtp".to_owned())
            .interact()?,
    }))
}

#[cfg(feature = "imap-backend")]
fn configure_imap(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
    use crate::account::DeserializedImapAccountConfig;
    use himalaya_lib::ImapConfig;

    let mut backend = ImapConfig::default();

    // TODO: Validate by checking as valid URI
    backend.host = Input::with_theme(&*THEME)
        .with_prompt("Enter the IMAP host:")
        .default(format!("imap.{}", base.email.rsplit_once('@').unwrap().1))
        .interact()?;

    backend.port = Input::with_theme(&*THEME)
        .with_prompt("Enter the IMAP port:")
        .validate_with(|input: &String| input.parse::<u16>().map(|_| ()))
        .default(993.to_string())
        .interact()
        .map(|input| input.parse::<u16>().unwrap())?;

    backend.login = Input::with_theme(&*THEME)
        .with_prompt("Enter your IMAP login:")
        .default(base.email.clone())
        .interact()?;

    backend.passwd_cmd = Input::with_theme(&*THEME)
        .with_prompt("What shell command should we run to get your password?")
        .default(format!("pass show {}", &base.email))
        .interact()?;

    match Select::with_theme(&*THEME)
        .with_prompt("Which security protocol do you want to use?")
        .items(&["TLS", "STARTTLS", "None"])
        .default(0)
        .interact_opt()?
    {
        Some(idx) if SECURITY_PROTOCOLS[idx] == "SSL/TLS" => backend.ssl = Some(true),
        Some(idx) if SECURITY_PROTOCOLS[idx] == "STARTTLS" => backend.starttls = Some(true),
        _ => {}
    };

    Ok(DeserializedAccountConfig::Imap(
        DeserializedImapAccountConfig { base, backend },
    ))
}

#[cfg(feature = "maildir-backend")]
fn configure_maildir(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
    use crate::account::DeserializedMaildirAccountConfig;
    use himalaya_lib::MaildirConfig;

    let backend = MaildirConfig::default();
    // TODO

    Ok(DeserializedAccountConfig::Maildir(
        DeserializedMaildirAccountConfig { base, backend },
    ))
}

#[cfg(feature = "notmuch-backend")]
fn configure_notmuch(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
    use crate::account::DeserializedNotmuchAccountConfig;
    use himalaya_lib::NotmuchConfig;

    let backend = NotmuchConfig::default();
    // TODO

    Ok(DeserializedAccountConfig::Notmuch(
        DeserializedNotmuchAccountConfig { base, backend },
    ))
}

fn configure_base() -> Result<DeserializedBaseAccountConfig> {
    let mut base_acc_config = DeserializedBaseAccountConfig::default();

    base_acc_config.email = Input::with_theme(&*THEME)
        .with_prompt("Enter your email:")
        .validate_with(EmailValidator)
        .interact()?;

    base_acc_config.display_name = Some(
        Input::with_theme(&*THEME)
            .with_prompt("Enter display name:")
            .interact()?,
    );

    Ok(base_acc_config)
}

struct EmailValidator;

impl<T: ToString> Validator<T> for EmailValidator {
    type Err = anyhow::Error;

    fn validate(&mut self, input: &T) -> Result<(), Self::Err> {
        let input = input.to_string();
        if EmailAddress::is_valid(&input) {
            Ok(())
        } else {
            Err(anyhow!("Invalid email address: {}", input))
        }
    }
}
