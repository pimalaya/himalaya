use color_eyre::{eyre::OptionExt, Result};
use email_address::EmailAddress;
use inquire::validator::{ErrorMessage, Validation};
use std::{path::PathBuf, str::FromStr};

use crate::wizard_warn;
use crate::{
    backend::{self, config::BackendConfig, BackendKind},
    message::config::{MessageConfig, MessageSendConfig},
};

use super::TomlAccountConfig;

pub(crate) async fn configure() -> Result<Option<(String, TomlAccountConfig)>> {
    let mut config = TomlAccountConfig {
        email: inquire::Text::new("Email address: ")
            .with_validator(|email: &_| {
                if EmailAddress::is_valid(email) {
                    Ok(Validation::Valid)
                } else {
                    Ok(Validation::Invalid(ErrorMessage::Custom(format!(
                        "Invalid email address: {email}"
                    ))))
                }
            })
            .prompt()?,

        ..Default::default()
    };

    let addr = EmailAddress::from_str(&config.email).unwrap();

    #[cfg(feature = "wizard")]
    let autoconfig_email = config.email.to_owned();
    #[cfg(feature = "wizard")]
    let autoconfig =
        tokio::spawn(async move { email::autoconfig::from_addr(&autoconfig_email).await.ok() });

    let account_name = inquire::Text::new("Account name: ")
        .with_default(
            addr.domain()
                .split_once('.')
                .ok_or_eyre("not a valid domain, without any .")?
                .0,
        )
        .prompt()?;

    config.display_name = Some(
        inquire::Text::new("Full display name: ")
            .with_default(addr.local_part())
            .prompt()?,
    );

    config.downloads_dir = Some(PathBuf::from(
        inquire::Text::new("Downloads directory: ")
            .with_default("~/Downloads")
            .prompt()?,
    ));

    let email = &config.email;
    #[cfg(feature = "wizard")]
    let autoconfig = autoconfig.await?;
    #[cfg(feature = "wizard")]
    let autoconfig = autoconfig.as_ref();

    #[cfg(feature = "wizard")]
    if let Some(config) = autoconfig {
        if config.is_gmail() {
            println!();
            wizard_warn!("Warning: Google passwords cannot be used directly, see:");
            wizard_warn!("https://pimalaya.org/himalaya/cli/latest/configuration/gmail.html");
            println!();
        }
    }

    match backend::wizard::configure(
        &account_name,
        email,
        #[cfg(feature = "wizard")]
        autoconfig,
    )
    .await?
    {
        #[cfg(feature = "imap")]
        Some(BackendConfig::Imap(imap_config)) => {
            config.imap = Some(imap_config);
            config.backend = Some(BackendKind::Imap);
        }
        #[cfg(feature = "maildir")]
        Some(BackendConfig::Maildir(mdir_config)) => {
            config.maildir = Some(mdir_config);
            config.backend = Some(BackendKind::Maildir);
        }
        #[cfg(feature = "notmuch")]
        Some(BackendConfig::Notmuch(notmuch_config)) => {
            config.notmuch = Some(notmuch_config);
            config.backend = Some(BackendKind::Notmuch);
        }
        _ => (),
    };

    match backend::wizard::configure_sender(
        &account_name,
        email,
        #[cfg(feature = "wizard")]
        autoconfig,
    )
    .await?
    {
        #[cfg(feature = "smtp")]
        Some(BackendConfig::Smtp(smtp_config)) => {
            config.smtp = Some(smtp_config);
            config.message = Some(MessageConfig {
                send: Some(MessageSendConfig {
                    backend: Some(BackendKind::Smtp),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        #[cfg(feature = "sendmail")]
        Some(BackendConfig::Sendmail(sendmail_config)) => {
            config.sendmail = Some(sendmail_config);
            config.message = Some(MessageConfig {
                send: Some(MessageSendConfig {
                    backend: Some(BackendKind::Sendmail),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        _ => (),
    };

    Ok(Some((account_name, config)))
}
