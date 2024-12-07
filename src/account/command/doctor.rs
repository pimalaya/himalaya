use std::{
    io::{stdout, Write},
    sync::Arc,
};

use clap::Parser;
use color_eyre::{Result, Section};
#[cfg(all(feature = "keyring", feature = "imap"))]
use email::imap::config::ImapAuthConfig;
#[cfg(feature = "imap")]
use email::imap::ImapContextBuilder;
#[cfg(feature = "maildir")]
use email::maildir::MaildirContextBuilder;
#[cfg(feature = "notmuch")]
use email::notmuch::NotmuchContextBuilder;
#[cfg(feature = "sendmail")]
use email::sendmail::SendmailContextBuilder;
#[cfg(all(feature = "keyring", feature = "smtp"))]
use email::smtp::config::SmtpAuthConfig;
#[cfg(feature = "smtp")]
use email::smtp::SmtpContextBuilder;
use email::{backend::BackendBuilder, config::Config};
#[cfg(feature = "keyring")]
use pimalaya_tui::terminal::prompt;
use pimalaya_tui::{
    himalaya::config::{Backend, SendingBackend},
    terminal::config::TomlConfig as _,
};

use crate::{account::arg::name::OptionalAccountNameArg, config::TomlConfig};

/// Diagnose and fix the given account.
///
/// This command diagnoses the given account and can even try to fix
/// it. It mostly checks if the configuration is valid, if backends
/// can be instanciated and if sessions work as expected.
#[derive(Debug, Parser)]
pub struct AccountDoctorCommand {
    #[command(flatten)]
    pub account: OptionalAccountNameArg,

    /// Try to fix the given account.
    ///
    /// This argument can be used to (re)configure keyring entries for
    /// example.
    #[arg(long, short)]
    pub fix: bool,
}

impl AccountDoctorCommand {
    pub async fn execute(self, config: &TomlConfig) -> Result<()> {
        let mut stdout = stdout();

        if let Some(name) = self.account.name.as_ref() {
            print!("Checking TOML configuration integrity for account {name}… ");
        } else {
            print!("Checking TOML configuration integrity for default account… ");
        }

        stdout.flush()?;

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref(), |c: &Config, name| {
                c.account(name).ok()
            })?;
        let account_config = Arc::new(account_config);

        println!("OK");

        #[cfg(feature = "keyring")]
        if self.fix {
            if prompt::bool("Would you like to reset existing keyring entries?", false)? {
                print!("Resetting keyring entries… ");
                stdout.flush()?;

                #[cfg(feature = "imap")]
                match toml_account_config.imap_auth_config() {
                    Some(ImapAuthConfig::Password(config)) => config.reset().await?,
                    #[cfg(feature = "oauth2")]
                    Some(ImapAuthConfig::OAuth2(config)) => config.reset().await?,
                    _ => (),
                }

                #[cfg(feature = "smtp")]
                match toml_account_config.smtp_auth_config() {
                    Some(SmtpAuthConfig::Password(config)) => config.reset().await?,
                    #[cfg(feature = "oauth2")]
                    Some(SmtpAuthConfig::OAuth2(config)) => config.reset().await?,
                    _ => (),
                }

                #[cfg(any(feature = "pgp-gpg", feature = "pgp-commands", feature = "pgp-native"))]
                if let Some(config) = &toml_account_config.pgp {
                    config.reset().await?;
                }

                println!("OK");
            }

            #[cfg(feature = "imap")]
            match toml_account_config.imap_auth_config() {
                Some(ImapAuthConfig::Password(config)) => {
                    config
                        .configure(|| Ok(prompt::password("IMAP password")?))
                        .await?;
                }
                #[cfg(feature = "oauth2")]
                Some(ImapAuthConfig::OAuth2(config)) => {
                    config
                        .configure(|| Ok(prompt::secret("IMAP OAuth 2.0 client secret")?))
                        .await?;
                }
                _ => (),
            };

            #[cfg(feature = "smtp")]
            match toml_account_config.smtp_auth_config() {
                Some(SmtpAuthConfig::Password(config)) => {
                    config
                        .configure(|| Ok(prompt::password("SMTP password")?))
                        .await?;
                }
                #[cfg(feature = "oauth2")]
                Some(SmtpAuthConfig::OAuth2(config)) => {
                    config
                        .configure(|| Ok(prompt::secret("SMTP OAuth 2.0 client secret")?))
                        .await?;
                }
                _ => (),
            };

            #[cfg(any(feature = "pgp-gpg", feature = "pgp-commands", feature = "pgp-native"))]
            if let Some(config) = &toml_account_config.pgp {
                config
                    .configure(&toml_account_config.email, || {
                        Ok(prompt::password("PGP secret key password")?)
                    })
                    .await?;
            }
        }

        match toml_account_config.backend {
            #[cfg(feature = "maildir")]
            Some(Backend::Maildir(mdir_config)) => {
                print!("Checking Maildir integrity… ");
                stdout.flush()?;

                let ctx = MaildirContextBuilder::new(account_config.clone(), Arc::new(mdir_config));
                BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await?;

                println!("OK");
            }
            #[cfg(feature = "imap")]
            Some(Backend::Imap(imap_config)) => {
                print!("Checking IMAP integrity… ");
                stdout.flush()?;

                let ctx = ImapContextBuilder::new(account_config.clone(), Arc::new(imap_config))
                    .with_pool_size(1);
                let res = BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await;

                if self.fix {
                    res?;
                } else {
                    res.note("Run with --fix to (re)configure your account.")?;
                }

                println!("OK");
            }
            #[cfg(feature = "notmuch")]
            Some(Backend::Notmuch(notmuch_config)) => {
                print!("Checking Notmuch integrity… ");
                stdout.flush()?;

                let ctx =
                    NotmuchContextBuilder::new(account_config.clone(), Arc::new(notmuch_config));
                BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await?;

                println!("OK");
            }
            _ => (),
        }

        let sending_backend = toml_account_config
            .message
            .and_then(|msg| msg.send)
            .and_then(|send| send.backend);

        match sending_backend {
            #[cfg(feature = "smtp")]
            Some(SendingBackend::Smtp(smtp_config)) => {
                print!("Checking SMTP integrity… ");
                stdout.flush()?;

                let ctx = SmtpContextBuilder::new(account_config.clone(), Arc::new(smtp_config));
                let res = BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await;

                if self.fix {
                    res?;
                } else {
                    res.note("Run with --fix to (re)configure your account.")?;
                }

                println!("OK");
            }
            #[cfg(feature = "sendmail")]
            Some(SendingBackend::Sendmail(sendmail_config)) => {
                print!("Checking Sendmail integrity… ");
                stdout.flush()?;

                let ctx =
                    SendmailContextBuilder::new(account_config.clone(), Arc::new(sendmail_config));
                BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await?;

                println!("OK");
            }
            _ => (),
        }

        Ok(())
    }
}
