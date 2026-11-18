//! # Microsoft Graph command
//!
//! The `msgraph` command, dispatching onto its subcommand groups.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    msgraph::{
        attachments::cli::MsgraphAttachmentsCommand, client::MsgraphClient,
        mail_folders::cli::MsgraphMailFoldersCommand, messages::cli::MsgraphMessagesCommand,
        profile::cli::MsgraphProfileCommand, request::MsgraphRequestCommand,
    },
    shared::refusal::{self, Reader},
};

/// The reader of what Graph refused.
const REFUSAL: Reader = crate::msgraph::client::refusal;

/// Microsoft Graph-specific API.
///
/// Each subcommand group tracks one Graph mail resource one to one.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MsgraphCommand {
    Request(MsgraphRequestCommand),
    #[command(subcommand)]
    Profile(MsgraphProfileCommand),
    #[command(subcommand, visible_alias = "folders", aliases = ["mail-folder", "folder"])]
    MailFolders(MsgraphMailFoldersCommand),
    #[command(subcommand, visible_alias = "msg", alias = "message")]
    Messages(MsgraphMessagesCommand),
    #[command(subcommand, alias = "attachment")]
    Attachments(MsgraphAttachmentsCommand),
}

impl MsgraphCommand {
    /// Runs the subcommand against the account's Graph client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let result = match self {
            Self::Profile(cmd) => cmd.execute(printer, account, client),
            Self::MailFolders(cmd) => cmd.execute(printer, account, client),
            Self::Messages(cmd) => cmd.execute(printer, account, client),
            Self::Attachments(cmd) => cmd.execute(printer, account, client),
            Self::Request(cmd) => cmd.execute(printer, client),
        };

        refusal::print(printer, &[REFUSAL], result)
    }
}

#[cfg(test)]
mod tests {
    use io_msgraph::v1::client::MsgraphClientStd;
    use pimalaya_cli::{error::PartialOutcome, printer::CapturePrinter};
    use serde_json::json;

    use crate::{
        account::context::Account,
        msgraph::{cli::MsgraphCommand, client::MsgraphClient, request::MsgraphRequestCommand},
        shared::http_script::{Script, refused},
    };

    /// A request Graph refuses is the command's result, under `refusal`,
    /// coded as a failure would be, and exits 3.
    #[test]
    fn a_refused_request_is_the_result() {
        let body = json!({
            "error": {"code": "ErrorAccessDenied", "message": "Access is denied."}
        });
        let (script, _) = Script::new(vec![refused(403, &body.to_string())]);
        let mut client =
            MsgraphClient::over(MsgraphClientStd::new(script, "token", Default::default()));
        let mut printer = CapturePrinter::default();

        let command = MsgraphCommand::Request(MsgraphRequestCommand {
            method: "get".into(),
            path: "me/mailFolders/nope".into(),
            body: None,
        });
        let result = command.execute(&mut printer, &mut Account::default(), &mut client);

        let err = result.unwrap_err();
        assert!(err.downcast_ref::<PartialOutcome>().is_some(), "{err}");
        let output = printer.into_single().unwrap();
        assert_eq!(output["refusal"]["status"], 403, "{output}");
        assert_eq!(output["refusal"]["reason"], "ErrorAccessDenied", "{output}");
        assert_eq!(
            output["refusal"]["message"], "Access is denied.",
            "{output}"
        );
        assert_eq!(output["refusal"]["code"], "permission-denied", "{output}");
    }
}
