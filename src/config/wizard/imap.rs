use super::{SECURITY_PROTOCOLS, THEME};
use crate::account::{
    DeserializedAccountConfig, DeserializedBaseAccountConfig, DeserializedImapAccountConfig,
};
use anyhow::Result;
use dialoguer::{Input, Select};
use himalaya_lib::ImapConfig;

#[cfg(feature = "imap-backend")]
pub(crate) fn configure(base: DeserializedBaseAccountConfig) -> Result<DeserializedAccountConfig> {
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
