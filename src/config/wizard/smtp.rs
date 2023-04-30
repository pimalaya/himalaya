use super::{SECURITY_PROTOCOLS, THEME};
use crate::account::DeserializedBaseAccountConfig;
use anyhow::Result;
use dialoguer::{Input, Select};
use pimalaya_email::{EmailSender, SmtpConfig};

pub(crate) fn configure(base: &DeserializedBaseAccountConfig) -> Result<EmailSender> {
    let mut smtp_config = SmtpConfig {
        host: Input::with_theme(&*THEME)
            .with_prompt("Enter the SMTP host: ")
            .default(format!("smtp.{}", base.email.rsplit_once('@').unwrap().1))
            .interact()?,
        ..Default::default()
    };

    let default_port = match Select::with_theme(&*THEME)
        .with_prompt("Which security protocol do you want to use?")
        .items(SECURITY_PROTOCOLS)
        .default(0)
        .interact_opt()?
    {
        Some(idx) if SECURITY_PROTOCOLS[idx] == "SSL/TLS" => {
            smtp_config.ssl = Some(true);
            465
        }
        Some(idx) if SECURITY_PROTOCOLS[idx] == "STARTTLS" => {
            smtp_config.starttls = Some(true);
            587
        }
        _ => 25,
    };

    smtp_config.port = Input::with_theme(&*THEME)
        .with_prompt("Enter the SMTP port:")
        .validate_with(|input: &String| input.parse::<u16>().map(|_| ()))
        .default(default_port.to_string())
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

    Ok(EmailSender::Smtp(smtp_config))
}
