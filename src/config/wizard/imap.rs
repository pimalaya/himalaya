use super::{SECURITY_PROTOCOLS, THEME};
use crate::account::{
    DeserializedAccountConfig, DeserializedBaseAccountConfig, DeserializedImapAccountConfig,
};
use anyhow::Result;
use dialoguer::{Input, Select};
use pimalaya_email::ImapConfig;

#[cfg(feature = "imap-backend")]
pub(crate) fn configure(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
    // TODO: Validate by checking as valid URI
    let mut backend = ImapConfig {
        host: Input::with_theme(&*THEME)
            .with_prompt("Enter the IMAP host:")
            .default(format!("imap.{}", base.email.rsplit_once('@').unwrap().1))
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
            backend.ssl = Some(true);
            993
        }
        Some(idx) if SECURITY_PROTOCOLS[idx] == "STARTTLS" => {
            backend.starttls = Some(true);
            143
        }
        _ => 143,
    };

    backend.port = Input::with_theme(&*THEME)
        .with_prompt("Enter the IMAP port:")
        .validate_with(|input: &String| input.parse::<u16>().map(|_| ()))
        .default(default_port.to_string())
        .interact()
        .map(|input| input.parse::<u16>().unwrap())?;

    backend.login = Input::with_theme(&*THEME)
        .with_prompt("Enter your IMAP login:")
        .default(base.email.clone())
        .interact()?;

    // FIXME: add all variants: password, password command and oauth2
    // backend.passwd_cmd = Input::with_theme(&*THEME)
    //     .with_prompt("What shell command should we run to get your password?")
    //     .default(format!("pass show {}", &base.email))
    //     .interact()?;

    Ok(DeserializedAccountConfig::Imap(
        DeserializedImapAccountConfig { base, backend },
    ))
}
