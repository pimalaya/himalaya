use anyhow::Result;
use dialoguer::{Input, Select};
use pimalaya_email::{BackendConfig, ImapConfig};

use crate::account::DeserializedAccountConfig;

use super::{AUTH_MECHANISMS, CMD, KEYRING, RAW, SECRET, SECURITY_PROTOCOLS, THEME};

#[cfg(feature = "imap-backend")]
pub(crate) fn configure(base: &DeserializedAccountConfig) -> Result<BackendConfig> {
    // TODO: Validate by checking as valid URI

    use dialoguer::Password;
    use pimalaya_email::{ImapAuthConfig, PasswdConfig};
    use pimalaya_secret::Secret;

    use super::PASSWD;

    let mut imap_config = ImapConfig::default();

    imap_config.host = Input::with_theme(&*THEME)
        .with_prompt("What is your IMAP host:")
        .default(format!("imap.{}", base.email.rsplit_once('@').unwrap().1))
        .interact()?;

    let default_port = match Select::with_theme(&*THEME)
        .with_prompt("Which security protocol do you want to use?")
        .items(SECURITY_PROTOCOLS)
        .default(0)
        .interact_opt()?
    {
        Some(idx) if SECURITY_PROTOCOLS[idx] == "SSL/TLS" => {
            imap_config.ssl = Some(true);
            993
        }
        Some(idx) if SECURITY_PROTOCOLS[idx] == "STARTTLS" => {
            imap_config.starttls = Some(true);
            143
        }
        _ => 143,
    };

    imap_config.port = Input::with_theme(&*THEME)
        .with_prompt("Which IMAP port would you like to use?")
        .validate_with(|input: &String| input.parse::<u16>().map(|_| ()))
        .default(default_port.to_string())
        .interact()
        .map(|input| input.parse::<u16>().unwrap())?;

    imap_config.login = Input::with_theme(&*THEME)
        .with_prompt("What is your IMAP login?")
        .default(base.email.clone())
        .interact()?;

    let auth = Select::with_theme(&*THEME)
        .with_prompt("Which IMAP authentication mechanism would you like to use?")
        .items(AUTH_MECHANISMS)
        .default(0)
        .interact_opt()?;

    imap_config.auth = match auth {
        Some(idx) if AUTH_MECHANISMS[idx] == PASSWD => {
            let secret = Select::with_theme(&*THEME)
                .with_prompt("How would you like to store your password?")
                .items(SECRET)
                .default(0)
                .interact_opt()?;
            match secret {
                Some(idx) if SECRET[idx] == RAW => ImapAuthConfig::Passwd(PasswdConfig {
                    passwd: Secret::new_raw(
                        Password::with_theme(&*THEME)
                            .with_prompt("What is your IMAP password?")
                            .interact()?,
                    ),
                }),
                _ => ImapAuthConfig::default(),
            }
        }
        _ => ImapAuthConfig::default(),
    };

    // FIXME: add all variants: password, password command and oauth2
    // backend.passwd_cmd = Input::with_theme(&*THEME)
    //     .with_prompt("What shell command should we run to get your password?")
    //     .default(format!("pass show {}", &base.email))
    //     .interact()?;

    Ok(BackendConfig::Imap(imap_config))
}
