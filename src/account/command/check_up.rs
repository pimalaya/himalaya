use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;
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
use email::{backend::BackendBuilder, config::Config};
use pimalaya_tui::{
    himalaya::config::{Backend, SendingBackend},
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
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

        printer.log("Checking configuration integrity…\n")?;

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref(), |c: &Config, name| {
                c.account(name).ok()
            })?;
        let account_config = Arc::new(account_config);

        match toml_account_config.backend {
            #[cfg(feature = "maildir")]
            Some(Backend::Maildir(mdir_config)) => {
                printer.log("Checking Maildir integrity…\n")?;

                let ctx = MaildirContextBuilder::new(account_config.clone(), Arc::new(mdir_config));
                BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await?;
            }
            #[cfg(feature = "imap")]
            Some(Backend::Imap(imap_config)) => {
                printer.log("Checking IMAP integrity…\n")?;

                let ctx = ImapContextBuilder::new(account_config.clone(), Arc::new(imap_config))
                    .with_pool_size(1);
                BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await?;
            }
            #[cfg(feature = "notmuch")]
            Some(Backend::Notmuch(notmuch_config)) => {
                printer.log("Checking Notmuch integrity…\n")?;

                let ctx =
                    NotmuchContextBuilder::new(account_config.clone(), Arc::new(notmuch_config));
                BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await?;
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
                printer.log("Checking SMTP integrity…\n")?;

                let ctx = SmtpContextBuilder::new(account_config.clone(), Arc::new(smtp_config));
                BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await?;
            }
            #[cfg(feature = "sendmail")]
            Some(SendingBackend::Sendmail(sendmail_config)) => {
                printer.log("Checking Sendmail integrity…\n")?;

                let ctx =
                    SendmailContextBuilder::new(account_config.clone(), Arc::new(sendmail_config));
                BackendBuilder::new(account_config.clone(), ctx)
                    .check_up()
                    .await?;
            }
            _ => (),
        }

        printer.out("Checkup successfully completed!\n")
    }
}
