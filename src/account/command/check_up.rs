use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
use email::backend::BackendBuilder;
#[cfg(feature = "imap")]
use email::imap::ImapContextBuilder;
#[cfg(feature = "maildir")]
use email::maildir::MaildirContextBuilder;
#[cfg(feature = "notmuch")]
use email::notmuch::NotmuchContextBuilder;
#[cfg(feature = "sendmail")]
use email::sendmail::SendmailContextBuilder;
#[cfg(feature = "smtp")]
use email::smtp::SmtpContextBuilder;
use pimalaya_tui::terminal::{cli::printer::Printer, config::TomlConfig as _};
use tracing::info;

use crate::{account::arg::name::OptionalAccountNameArg, config::TomlConfig};

/// Check up the given account.
///
/// This command performs a checkup of the given account. It checks if
/// the configuration is valid, if backend can be created and if
/// sessions work as expected.
#[derive(Debug, Parser)]
pub struct AccountCheckUpCommand {
    #[command(flatten)]
    pub account: OptionalAccountNameArg,
}

impl AccountCheckUpCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing check up account command");

        let account = self.account.name.as_ref().map(String::as_str);

        printer.log("Checking configuration integrity…")?;

        let (toml_account_config, account_config) = config.clone().into_account_configs(account)?;
        let account_config = Arc::new(account_config);

        printer.log("Checking backend context integrity…")?;

        #[cfg(feature = "maildir")]
        if let Some(mdir_config) = toml_account_config.maildir {
            printer.log("Checking Maildir integrity…")?;

            let ctx = MaildirContextBuilder::new(account_config.clone(), Arc::new(mdir_config));
            BackendBuilder::new(account_config.clone(), ctx)
                .check_up()
                .await?;
        }

        #[cfg(feature = "imap")]
        if let Some(imap_config) = toml_account_config.imap {
            printer.log("Checking IMAP integrity…")?;

            let ctx = ImapContextBuilder::new(account_config.clone(), Arc::new(imap_config))
                .with_pool_size(1);
            BackendBuilder::new(account_config.clone(), ctx)
                .check_up()
                .await?;
        }

        #[cfg(feature = "notmuch")]
        if let Some(notmuch_config) = toml_account_config.notmuch {
            printer.log("Checking Notmuch integrity…")?;

            let ctx = NotmuchContextBuilder::new(account_config.clone(), Arc::new(notmuch_config));
            BackendBuilder::new(account_config.clone(), ctx)
                .check_up()
                .await?;
        }

        #[cfg(feature = "smtp")]
        if let Some(smtp_config) = toml_account_config.smtp {
            printer.log("Checking SMTP integrity…")?;

            let ctx = SmtpContextBuilder::new(account_config.clone(), Arc::new(smtp_config));
            BackendBuilder::new(account_config.clone(), ctx)
                .check_up()
                .await?;
        }

        #[cfg(feature = "sendmail")]
        if let Some(sendmail_config) = toml_account_config.sendmail {
            printer.log("Checking Sendmail integrity…")?;

            let ctx =
                SendmailContextBuilder::new(account_config.clone(), Arc::new(sendmail_config));
            BackendBuilder::new(account_config.clone(), ctx)
                .check_up()
                .await?;
        }

        printer.out("Checkup successfully completed!")
    }
}
