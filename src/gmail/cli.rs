//! # Gmail command
//!
//! The `gmail` command, dispatching onto its subcommand groups.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    gmail::{
        attachments::GmailAttachmentsCommand, client::GmailClient, drafts::GmailDraftsCommand,
        history::GmailHistoryCommand, labels::GmailLabelsCommand, messages::GmailMessagesCommand,
        profile::GmailProfileCommand, request::GmailRequestCommand, settings::GmailSettingsCommand,
        threads::GmailThreadsCommand,
    },
    refusal::{self, Reader},
};

/// The reader of what Gmail refused.
const REFUSAL: Reader = crate::gmail::client::refusal;

/// Gmail-specific API.
///
/// Each subcommand group tracks one Gmail REST resource one to one.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailCommand {
    Request(GmailRequestCommand),
    #[command(subcommand)]
    Profile(GmailProfileCommand),
    #[command(subcommand)]
    #[command(alias = "label")]
    Labels(GmailLabelsCommand),
    #[command(subcommand)]
    #[command(visible_alias = "msg", alias = "message")]
    Messages(GmailMessagesCommand),
    #[command(subcommand)]
    #[command(alias = "attachment")]
    Attachments(GmailAttachmentsCommand),
    #[command(subcommand)]
    #[command(alias = "draft")]
    Drafts(GmailDraftsCommand),
    #[command(subcommand)]
    #[command(alias = "thread")]
    Threads(GmailThreadsCommand),
    #[command(subcommand)]
    History(GmailHistoryCommand),
    #[command(subcommand)]
    #[command(alias = "setting")]
    Settings(GmailSettingsCommand),
}

impl GmailCommand {
    /// Refuses what the command cannot run before the session opens: a
    /// `request` URL off the Gmail API.
    pub fn check(&self) -> Result<()> {
        match self {
            Self::Request(cmd) => cmd.check(),
            _ => Ok(()),
        }
    }

    /// Runs the subcommand against the account's Gmail client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let result = match self {
            Self::Profile(cmd) => cmd.execute(printer, account, client),
            Self::Labels(cmd) => cmd.execute(printer, account, client),
            Self::Messages(cmd) => cmd.execute(printer, account, client),
            Self::Attachments(cmd) => cmd.execute(printer, account, client),
            Self::Drafts(cmd) => cmd.execute(printer, account, client),
            Self::Threads(cmd) => cmd.execute(printer, account, client),
            Self::History(cmd) => cmd.execute(printer, account, client),
            Self::Settings(cmd) => cmd.execute(printer, account, client),
            Self::Request(cmd) => cmd.execute(printer, client),
        };

        refusal::print(printer, &[REFUSAL], result)
    }
}

#[cfg(test)]
mod tests {
    use io_gmail::v1::client::GmailClientStd;
    use pimalaya_cli::{error::PartialOutcome, printer::CapturePrinter};
    use serde_json::json;

    use crate::{
        account::context::Account,
        gmail::{cli::GmailCommand, client::GmailClient, request::GmailRequestCommand},
        http_script::{Script, refused},
    };

    /// A request Gmail refuses is the command's result, under `refusal`,
    /// and exits 3.
    #[test]
    fn a_refused_request_is_the_result() {
        let body = json!({
            "error": {
                "code": 404,
                "message": "Requested entity was not found.",
                "errors": [{"reason": "notFound"}],
                "status": "NOT_FOUND",
            }
        });
        let (script, _) = Script::new(vec![refused(404, &body.to_string())]);
        let mut client =
            GmailClient::over(GmailClientStd::new(script, "token", Default::default()));
        let mut printer = CapturePrinter::default();

        let command = GmailCommand::Request(GmailRequestCommand {
            method: "get".into(),
            path: "users/me/labels/nope".into(),
            body: None,
        });
        let result = command.execute(&mut printer, &mut Account::default(), &mut client);

        let err = result.unwrap_err();
        assert!(err.downcast_ref::<PartialOutcome>().is_some(), "{err}");
        let refusal = json!({
            "refusal": {
                "status": 404,
                "reason": "notFound",
                "message": "Requested entity was not found.",
                "code": null,
                "retryAfter": null,
            }
        });
        assert_eq!(printer.into_single().unwrap(), refusal);
    }
}
