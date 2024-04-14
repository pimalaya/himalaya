use clap::Parser;
use color_eyre::Result;
#[cfg(feature = "imap")]
use email::imap::config::ImapAuthConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::SmtpAuthConfig;
use tracing::info;
#[cfg(any(feature = "imap", feature = "smtp"))]
use tracing::{debug, warn};

#[cfg(any(feature = "imap", feature = "smtp", feature = "pgp"))]
use crate::ui::prompt;
use crate::{account::arg::name::AccountNameArg, config::TomlConfig, printer::Printer};

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
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing configure account command");

        let account = &self.account.name;
        let (_, account_config) = config.into_toml_account_config(Some(account))?;

        if self.reset {
            #[cfg(feature = "imap")]
            if let Some(ref config) = account_config.imap {
                let reset = match &config.auth {
                    ImapAuthConfig::Passwd(config) => config.reset().await,
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
                    config.configure(|| prompt::passwd("IMAP password")).await
                }
                ImapAuthConfig::OAuth2(config) => {
                    config
                        .configure(|| prompt::secret("IMAP OAuth 2.0 client secret"))
                        .await
                }
            }?;
        }

        #[cfg(feature = "smtp")]
        if let Some(ref config) = account_config.smtp {
            match &config.auth {
                SmtpAuthConfig::Passwd(config) => {
                    config.configure(|| prompt::passwd("SMTP password")).await
                }
                SmtpAuthConfig::OAuth2(config) => {
                    config
                        .configure(|| prompt::secret("SMTP OAuth 2.0 client secret"))
                        .await
                }
            }?;
        }

        #[cfg(feature = "pgp")]
        if let Some(ref config) = account_config.pgp {
            config
                .configure(&account_config.email, || {
                    prompt::passwd("PGP secret key password")
                })
                .await?;
        }

        printer.print(format!(
            "Account {account} successfully {}configured!",
            if self.reset { "re" } else { "" }
        ))
    }
}
