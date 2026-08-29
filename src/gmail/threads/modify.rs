//! # Gmail thread modify
//!
//! The `gmail threads modify` command, `users.threads.modify`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::threads::modify::GmailThreadModify;
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Modify the labels of every message in a Gmail thread
/// (users.threads.modify).
#[derive(Debug, Parser)]
pub struct GmailThreadModifyCommand {
    /// The id of the thread to modify.
    #[arg(value_name = "ID")]
    pub id: String,
    /// Label id to add to the thread. Can be repeated.
    #[arg(long = "add-label", value_name = "ID")]
    pub add: Vec<String>,
    /// Label id to remove from the thread. Can be repeated.
    #[arg(long = "remove-label", value_name = "ID")]
    pub remove: Vec<String>,
}

impl GmailThreadModifyCommand {
    /// Applies the label changes to every message of the thread.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailThreadModify::new(
                &client.auth,
                &client.user_id,
                &self.id,
                &self.add,
                &self.remove,
            )?;
            client.run(c)?
        };
        let thread = out.response;

        printer.out(Message::new(format!(
            "Gmail thread `{}` successfully modified",
            thread.id
        )))
    }
}
