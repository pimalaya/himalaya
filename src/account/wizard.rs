use anyhow::{bail, Result};
use dialoguer::{Confirm, Input};
use email::account::sync::config::SyncConfig;
use email_address::EmailAddress;

use crate::{
    backend::{self, config::BackendConfig, BackendKind},
    config::wizard::THEME,
    message::config::{MessageConfig, MessageSendConfig},
    wizard_prompt,
};

use super::TomlAccountConfig;

pub(crate) async fn configure() -> Result<Option<(String, TomlAccountConfig)>> {
    let mut config = TomlAccountConfig::default();

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
                bail!("Invalid email address: {email}")
            }
        })
        .interact()?;

    config.display_name = Some(
        Input::with_theme(&*THEME)
            .with_prompt("Full display name")
            .interact()?,
    );

    config.downloads_dir = Some(
        Input::with_theme(&*THEME)
            .with_prompt("Downloads directory")
            .default(String::from("~/Downloads"))
            .interact()?
            .into(),
    );

    match backend::wizard::configure(&account_name, &config.email).await? {
        Some(BackendConfig::Maildir(mdir_config)) => {
            config.maildir = Some(mdir_config);
            config.backend = Some(BackendKind::Maildir);
        }
        #[cfg(feature = "imap")]
        Some(BackendConfig::Imap(imap_config)) => {
            config.imap = Some(imap_config);
            config.backend = Some(BackendKind::Imap);
        }
        #[cfg(feature = "notmuch")]
        Some(BackendConfig::Notmuch(notmuch_config)) => {
            config.notmuch = Some(notmuch_config);
            config.backend = Some(BackendKind::Notmuch);
        }
        _ => (),
    };

    match backend::wizard::configure_sender(&account_name, &config.email).await? {
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
        _ => (),
    };

    let should_configure_sync = Confirm::new()
        .with_prompt(wizard_prompt!(
            "Do you need an offline access to your account?"
        ))
        .default(false)
        .interact_opt()?
        .unwrap_or_default();

    if should_configure_sync {
        config.sync = Some(SyncConfig {
            enable: Some(true),
            ..Default::default()
        });
    }

    Ok(Some((account_name, config)))
}
