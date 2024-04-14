use clap::Parser;
use color_eyre::Result;
use email::backend::context::BackendContextBuilder;
use tracing::info;

use crate::{
    account::arg::name::OptionalAccountNameArg, backend, config::TomlConfig, printer::Printer,
};

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

        printer.print_log("Checking configuration integrity…")?;

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            account,
            #[cfg(feature = "account-sync")]
            true,
        )?;
        let used_backends = toml_account_config.get_used_backends();

        printer.print_log("Checking backend context integrity…")?;

        let ctx_builder = backend::BackendContextBuilder::new(
            toml_account_config.clone(),
            account_config,
            Vec::from_iter(used_backends),
        )
        .await?;

        let ctx = ctx_builder.clone().build().await?;

        #[cfg(feature = "maildir")]
        {
            printer.print_log("Checking Maildir integrity…")?;

            let maildir = ctx_builder
                .maildir
                .as_ref()
                .and_then(|maildir| maildir.check_up())
                .and_then(|f| ctx.maildir.as_ref().and_then(|ctx| f(ctx)));

            if let Some(maildir) = maildir.as_ref() {
                maildir.check_up().await?;
            }
        }

        #[cfg(feature = "imap")]
        {
            printer.print_log("Checking IMAP integrity…")?;

            let imap = ctx_builder
                .imap
                .as_ref()
                .and_then(|imap| imap.check_up())
                .and_then(|f| ctx.imap.as_ref().and_then(|ctx| f(ctx)));

            if let Some(imap) = imap.as_ref() {
                imap.check_up().await?;
            }
        }

        #[cfg(feature = "notmuch")]
        {
            printer.print_log("Checking Notmuch integrity…")?;

            let notmuch = ctx_builder
                .notmuch
                .as_ref()
                .and_then(|notmuch| notmuch.check_up())
                .and_then(|f| ctx.notmuch.as_ref().and_then(|ctx| f(ctx)));

            if let Some(notmuch) = notmuch.as_ref() {
                notmuch.check_up().await?;
            }
        }

        #[cfg(feature = "smtp")]
        {
            printer.print_log("Checking SMTP integrity…")?;

            let smtp = ctx_builder
                .smtp
                .as_ref()
                .and_then(|smtp| smtp.check_up())
                .and_then(|f| ctx.smtp.as_ref().and_then(|ctx| f(ctx)));

            if let Some(smtp) = smtp.as_ref() {
                smtp.check_up().await?;
            }
        }

        #[cfg(feature = "sendmail")]
        {
            printer.print_log("Checking Sendmail integrity…")?;

            let sendmail = ctx_builder
                .sendmail
                .as_ref()
                .and_then(|sendmail| sendmail.check_up())
                .and_then(|f| ctx.sendmail.as_ref().and_then(|ctx| f(ctx)));

            if let Some(sendmail) = sendmail.as_ref() {
                sendmail.check_up().await?;
            }
        }

        printer.print("Checkup successfully completed!")
    }
}
