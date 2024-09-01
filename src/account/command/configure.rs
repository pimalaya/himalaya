use clap::Parser;
use color_eyre::Result;
#[cfg(feature = "imap")]
use email::imap::config::ImapAuthConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::SmtpAuthConfig;
#[cfg(any(feature = "imap", feature = "smtp", feature = "pgp"))]
use pimalaya_tui::prompt;
use tracing::info;
#[cfg(any(feature = "imap", feature = "smtp"))]
use tracing::{debug, warn};

use crate::{account::arg::name::AccountNameArg, config::Config, printer::Printer};

/// Configure an account.
///
/// This command is mostly used to define or reset passwords managed
/// by your global keyring. If you do not use the keyring system, you
/// can skip this command.
#[derive(Debug, Parser)]
pub struct AccountConfigureCommand {
    #[command(flatten)]
    pub account: AccountNameArg,

    /// Reset keyring passwords.
    ///
    /// This argument will force passwords to be prompted again, then
    /// saved to your global keyring.
    #[arg(long, short)]
    pub reset: bool,
}

impl AccountConfigureCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &Config) -> Result<()> {
        info!("executing configure account command");

        let account = &self.account.name;
        let (_, account_config) = config.into_toml_account_config(Some(account))?;

        if self.reset {
            #[cfg(feature = "imap")]
            if let Some(ref config) = account_config.imap {
                let reset = match &config.auth {
                    ImapAuthConfig::Passwd(config) => config.reset().await,
                    #[cfg(feature = "oauth2")]
                    ImapAuthConfig::OAuth2(config) => config.reset().await,
                };
                if let Err(err) = reset {
                    warn!("error while resetting imap secrets: {err}");
                    debug!("error while resetting imap secrets: {err:?}");
                }
            }

            #[cfg(feature = "smtp")]
            if let Some(ref config) = account_config.smtp {
                let reset = match &config.auth {
                    SmtpAuthConfig::Passwd(config) => config.reset().await,
                    #[cfg(feature = "oauth2")]
                    SmtpAuthConfig::OAuth2(config) => config.reset().await,
                };
                if let Err(err) = reset {
                    warn!("error while resetting smtp secrets: {err}");
                    debug!("error while resetting smtp secrets: {err:?}");
                }
            }

            #[cfg(feature = "pgp")]
            if let Some(ref config) = account_config.pgp {
                config.reset().await?;
            }
        }

        #[cfg(feature = "imap")]
        if let Some(ref config) = account_config.imap {
            match &config.auth {
                ImapAuthConfig::Passwd(config) => {
                    config
                        .configure(|| Ok(prompt::password("IMAP password")?))
                        .await
                }
                #[cfg(feature = "oauth2")]
                ImapAuthConfig::OAuth2(config) => {
                    config
                        .configure(|| Ok(prompt::secret("IMAP OAuth 2.0 clientsecret")?))
                        .await
                }
            }?;
        }

        #[cfg(feature = "smtp")]
        if let Some(ref config) = account_config.smtp {
            match &config.auth {
                SmtpAuthConfig::Passwd(config) => {
                    config
                        .configure(|| Ok(prompt::password("SMTP password")?))
                        .await
                }
                #[cfg(feature = "oauth2")]
                SmtpAuthConfig::OAuth2(config) => {
                    config
                        .configure(|| Ok(prompt::secret("SMTP OAuth 2.0 client secret")?))
                        .await
                }
            }?;
        }

        #[cfg(feature = "pgp")]
        if let Some(ref config) = account_config.pgp {
            config
                .configure(&account_config.email, || {
                    Ok(prompt::password("PGP secret key password")?)
                })
                .await?;
        }

        printer.out(format!(
            "Account {account} successfully {}configured!",
            if self.reset { "re" } else { "" }
        ))
    }
}
