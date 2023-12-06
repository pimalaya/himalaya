use anyhow::Result;
use clap::Parser;
#[cfg(feature = "imap")]
use email::imap::config::ImapAuthConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::SmtpAuthConfig;
use log::{debug, info, warn};

use crate::{
    config::{
        wizard::{prompt_passwd, prompt_secret},
        TomlConfig,
    },
    printer::Printer,
};

/// Configure the given account
#[derive(Debug, Parser)]
pub struct Command {
    /// The name of the account that needs to be configured
    ///
    /// The account names are taken from the table at the root level
    /// of your TOML configuration file.
    #[arg(value_name = "NAME")]
    pub account_name: String,

    /// Force the account to reconfigure, even if it is already
    /// configured
    #[arg(long, short)]
    pub force: bool,
}

impl Command {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing account configure command");

        let (_, account_config) =
            config.into_toml_account_config(Some(self.account_name.as_str()))?;

        if self.force {
            #[cfg(feature = "imap")]
            if let Some(ref config) = account_config.imap {
                let reset = match &config.auth {
                    ImapAuthConfig::Passwd(config) => config.reset(),
                    ImapAuthConfig::OAuth2(config) => config.reset(),
                };
                if let Err(err) = reset {
                    warn!("error while resetting imap secrets: {err}");
                    debug!("error while resetting imap secrets: {err:?}");
                }
            }

            #[cfg(feature = "smtp")]
            if let Some(ref config) = account_config.smtp {
                let reset = match &config.auth {
                    SmtpAuthConfig::Passwd(config) => config.reset(),
                    SmtpAuthConfig::OAuth2(config) => config.reset(),
                };
                if let Err(err) = reset {
                    warn!("error while resetting smtp secrets: {err}");
                    debug!("error while resetting smtp secrets: {err:?}");
                }
            }

            #[cfg(feature = "pgp")]
            if let Some(ref config) = account_config.pgp {
                account_config.pgp.reset().await?;
            }
        }

        #[cfg(feature = "imap")]
        if let Some(ref config) = account_config.imap {
            match &config.auth {
                ImapAuthConfig::Passwd(config) => {
                    config.configure(|| prompt_passwd("IMAP password")).await
                }
                ImapAuthConfig::OAuth2(config) => {
                    config
                        .configure(|| prompt_secret("IMAP OAuth 2.0 client secret"))
                        .await
                }
            }?;
        }

        #[cfg(feature = "smtp")]
        if let Some(ref config) = account_config.smtp {
            match &config.auth {
                SmtpAuthConfig::Passwd(config) => {
                    config.configure(|| prompt_passwd("SMTP password")).await
                }
                SmtpAuthConfig::OAuth2(config) => {
                    config
                        .configure(|| prompt_secret("SMTP OAuth 2.0 client secret"))
                        .await
                }
            }?;
        }

        #[cfg(feature = "pgp")]
        if let Some(ref config) = config.pgp {
            config
                .pgp
                .configure(&config.email, || prompt_passwd("PGP secret key password"))
                .await?;
        }

        printer.print(format!(
            "Account {} successfully {}configured!",
            self.account_name,
            if self.force { "re" } else { "" }
        ))?;

        Ok(())
    }
}
