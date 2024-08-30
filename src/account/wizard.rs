use color_eyre::Result;
use pimalaya_tui::{print, prompt};

use crate::{
    backend::{self, config::BackendConfig, BackendKind},
    message::config::{MessageConfig, MessageSendConfig},
};

use super::TomlAccountConfig;

pub async fn configure() -> Result<(String, TomlAccountConfig)> {
    let email = prompt::email("Email address:", None)?;

    let mut config = TomlAccountConfig {
        email: email.to_string(),
        ..Default::default()
    };

    let autoconfig_email = config.email.to_owned();
    let autoconfig =
        tokio::spawn(async move { email::autoconfig::from_addr(&autoconfig_email).await.ok() });

    let default_account_name = email
        .domain()
        .split_once('.')
        .map(|domain| domain.0)
        .unwrap_or(email.domain());
    let account_name = prompt::text("Account name:", Some(default_account_name))?;

    config.display_name = Some(prompt::text(
        "Full display name:",
        Some(email.local_part()),
    )?);

    config.downloads_dir = Some(prompt::path("Downloads directory:", Some("~/Downloads"))?);

    let autoconfig = autoconfig.await?;
    let autoconfig = autoconfig.as_ref();

    if let Some(config) = autoconfig {
        if config.is_gmail() {
            println!();
            print::warn("Warning: Google passwords cannot be used directly, see:");
            print::warn("https://github.com/pimalaya/himalaya?tab=readme-ov-file#configuration");
            println!();
        }
    }

    match backend::wizard::configure(&account_name, &email, autoconfig).await? {
        #[cfg(feature = "imap")]
        BackendConfig::Imap(imap_config) => {
            config.imap = Some(imap_config);
            config.backend = Some(BackendKind::Imap);
        }
        #[cfg(feature = "maildir")]
        BackendConfig::Maildir(mdir_config) => {
            config.maildir = Some(mdir_config);
            config.backend = Some(BackendKind::Maildir);
        }
        #[cfg(feature = "notmuch")]
        BackendConfig::Notmuch(notmuch_config) => {
            config.notmuch = Some(notmuch_config);
            config.backend = Some(BackendKind::Notmuch);
        }
        _ => unreachable!(),
    };

    match backend::wizard::configure_sender(&account_name, &email, autoconfig).await? {
        #[cfg(feature = "smtp")]
        BackendConfig::Smtp(smtp_config) => {
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
        BackendConfig::Sendmail(sendmail_config) => {
            config.sendmail = Some(sendmail_config);
            config.message = Some(MessageConfig {
                send: Some(MessageSendConfig {
                    backend: Some(BackendKind::Sendmail),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        _ => unreachable!(),
    };

    Ok((account_name, config))
}
