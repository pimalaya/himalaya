use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::filters::{
    GmailFilter, GmailFilterAction, GmailFilterCriteria, create::GmailFilterCreate,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::client::GmailClient;

/// Create a Gmail filter (users.settings.filters.create).
#[derive(Debug, Parser)]
pub struct GmailSettingsFilterCreateCommand {
    /// Match messages whose sender matches this value.
    #[arg(long, value_name = "ADDR")]
    pub from: Option<String>,
    /// Match messages whose recipient matches this value.
    #[arg(long, value_name = "ADDR")]
    pub to: Option<String>,
    /// Match messages whose subject matches this value.
    #[arg(long, value_name = "TEXT")]
    pub subject: Option<String>,
    /// Match messages with this Gmail search query.
    #[arg(long, value_name = "QUERY")]
    pub query: Option<String>,
    /// Exclude messages matching this Gmail search query.
    #[arg(long, value_name = "QUERY")]
    pub negated_query: Option<String>,
    /// Match only messages that have an attachment.
    #[arg(long)]
    pub has_attachment: bool,
    /// Label identifier to add to matching messages (repeatable).
    #[arg(long = "add-label", value_name = "ID")]
    pub add_label: Vec<String>,
    /// Label identifier to remove from matching messages (repeatable).
    #[arg(long = "remove-label", value_name = "ID")]
    pub remove_label: Vec<String>,
    /// Forward matching messages to this address.
    #[arg(long, value_name = "ADDR")]
    pub forward: Option<String>,
}

impl GmailSettingsFilterCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let filter = GmailFilter {
            id: String::new(),
            criteria: Some(GmailFilterCriteria {
                from: self.from,
                to: self.to,
                subject: self.subject,
                query: self.query,
                negated_query: self.negated_query,
                has_attachment: self.has_attachment.then_some(true),
                exclude_chats: None,
                size: None,
                size_comparison: None,
            }),
            action: Some(GmailFilterAction {
                add_label_ids: (!self.add_label.is_empty()).then(|| self.add_label.clone()),
                remove_label_ids: (!self.remove_label.is_empty())
                    .then(|| self.remove_label.clone()),
                forward: self.forward,
            }),
        };

        let out = {
            let c = GmailFilterCreate::new(&client.auth, &client.user_id, &filter)?;
            client.run(c)?
        };
        let created = out.response;

        printer.out(Message::new(format!(
            "Gmail filter `{}` successfully created",
            created.id
        )))
    }
}
