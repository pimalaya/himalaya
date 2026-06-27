use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use io_gmail::v1::rest::history::{
    GmailHistoryType,
    list::{GmailHistoryList, GmailHistoryListParams},
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{account::context::Account, gmail::client::GmailClient, shared::output::Paginated};

/// Manage the Gmail mailbox history (users.history).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailHistoryCommand {
    List(GmailHistoryListCommand),
}

impl GmailHistoryCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List the changes applied to the mailbox since a given history id.
#[derive(Debug, Parser)]
pub struct GmailHistoryListCommand {
    /// History id to start listing changes from.
    #[arg(long = "start-history-id", value_name = "ID")]
    pub start_history_id: String,

    /// Restrict the listing to changes affecting this label id.
    #[arg(long = "label-id", value_name = "ID")]
    pub label_id: Option<String>,

    /// History change types to include (repeatable).
    #[arg(long = "history-type", value_name = "TYPE")]
    pub history_types: Vec<HistoryTypeArg>,

    /// Maximum number of history records to return.
    #[arg(short = 's', long, value_name = "N")]
    pub max_results: Option<u32>,

    /// Page token from a previous listing, for pagination.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<String>,
}

impl GmailHistoryListCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let types: Vec<GmailHistoryType> =
            self.history_types.iter().copied().map(Into::into).collect();

        let out = {
            let params = GmailHistoryListParams {
                start_history_id: &self.start_history_id,
                label_id: self.label_id.as_deref(),
                history_types: &types,
                max_results: self.max_results,
                page_token: self.page_token.as_deref(),
            };
            let c = GmailHistoryList::new(&client.auth, &client.user_id, &params)?;
            client.run(c)?
        };

        let resp = out.response;

        let mut out_string = format!(
            "New history id: {}\n",
            resp.history_id.as_deref().unwrap_or("(none)")
        );

        if resp.history.is_empty() {
            out_string.push_str("No history changes since the given history id.");
        } else {
            for h in &resp.history {
                out_string.push_str(&format!(
                    "{}: +{}msg -{}msg +{}lbl -{}lbl\n",
                    h.id,
                    h.messages_added.len(),
                    h.messages_deleted.len(),
                    h.labels_added.len(),
                    h.labels_removed.len(),
                ));
            }
        }

        printer.out(Paginated::new(
            Message::new(out_string),
            resp.next_page_token,
        ))
    }
}

/// Gmail history change type accepted on the CLI.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "camelCase")]
pub enum HistoryTypeArg {
    MessageAdded,
    MessageDeleted,
    LabelAdded,
    LabelRemoved,
}

impl From<HistoryTypeArg> for GmailHistoryType {
    fn from(arg: HistoryTypeArg) -> Self {
        match arg {
            HistoryTypeArg::MessageAdded => GmailHistoryType::MessageAdded,
            HistoryTypeArg::MessageDeleted => GmailHistoryType::MessageDeleted,
            HistoryTypeArg::LabelAdded => GmailHistoryType::LabelAdded,
            HistoryTypeArg::LabelRemoved => GmailHistoryType::LabelRemoved,
        }
    }
}
