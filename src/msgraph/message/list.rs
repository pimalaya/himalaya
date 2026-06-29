use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_msgraph::v1::rest::users::messages::{
    MsgraphMessage, MsgraphRecipient, list::MsgraphMessagesListParams,
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::{account::context::Account, msgraph::client::MsgraphClient, shared::output::Paginated};

/// List Microsoft Graph messages (`GET /me/messages` or, with
/// `--folder`, `GET /me/mailFolders/{id}/messages`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageListCommand {
    /// Restrict the listing to a folder id or well-known name (e.g.
    /// `inbox`). Lists the whole mailbox when omitted.
    #[arg(short = 'f', long, value_name = "ID")]
    pub folder: Option<String>,

    /// Maximum number of messages to return (OData `$top`).
    #[arg(long, value_name = "N")]
    pub top: Option<u32>,

    /// Number of messages to skip (OData `$skip`).
    #[arg(long, value_name = "N")]
    pub skip: Option<u32>,

    /// OData `$filter` expression (e.g. `isRead eq false`).
    #[arg(long, value_name = "EXPR")]
    pub filter: Option<String>,

    /// OData `$search` query (e.g. `subject:report` or a bare term).
    ///
    /// Graph forbids combining `$search` with `$orderby` and ignores
    /// `$count`, so both are dropped when this is set; results come back
    /// in relevance order.
    #[arg(long, value_name = "QUERY")]
    pub search: Option<String>,

    /// OData `$orderby` expression. Defaults to `receivedDateTime desc`.
    #[arg(long, value_name = "EXPR")]
    pub orderby: Option<String>,

    /// OData `$select`: comma-separated fields to return (e.g.
    /// `subject,from,receivedDateTime`).
    #[arg(long, value_name = "FIELDS")]
    pub select: Option<String>,

    /// Request the total count of matching messages (OData `$count`).
    #[arg(long)]
    pub count: bool,
}

impl MsgraphMessageListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let search = self.search.as_deref();

        // Graph rejects `$search` alongside `$orderby` and needs a
        // `ConsistencyLevel` header (not sent by the client) for
        // `$search` + `$count`; drop both when searching.
        let orderby = match search {
            Some(_) => None,
            None => Some(self.orderby.as_deref().unwrap_or("receivedDateTime desc")),
        };
        let count = match search {
            Some(_) => None,
            None => self.count.then_some(true),
        };

        let params = MsgraphMessagesListParams {
            top: self.top,
            skip: self.skip,
            select: self.select.as_deref(),
            filter: self.filter.as_deref(),
            search,
            orderby,
            count,
        };
        let response = client
            .messages_list(self.folder.as_deref(), &params)?
            .response;

        let next_page = response.next_link;
        let table = messages_table(account, response.value);

        printer.out(Paginated::new(table, next_page))
    }
}

/// Renders the display form of a recipient: `Name <addr>`, or just the
/// address when there is no display name.
pub(crate) fn recipient(recipient: &MsgraphRecipient) -> String {
    let address = recipient.email_address.address.clone().unwrap_or_default();
    match &recipient.email_address.name {
        Some(name) if !name.is_empty() => format!("{name} <{address}>"),
        _ => address,
    }
}

/// Builds the renderable messages table with the account's
/// envelope-table colors and layout.
fn messages_table(account: &Account, messages: Vec<MsgraphMessage>) -> MessagesTable {
    MessagesTable {
        preset: account.table_preset().to_string(),
        arrangement: account.table_arrangement(),
        colors: MessageColors {
            id: account.envelopes_list_table_id_color(),
            subject: account.envelopes_list_table_subject_color(),
            from: account.envelopes_list_table_from_color(),
            date: account.envelopes_list_table_date_color(),
        },
        messages,
    }
}

/// Per-column colors for the Microsoft Graph messages table.
#[derive(Clone, Copy, Debug)]
pub struct MessageColors {
    pub id: Color,
    pub subject: Color,
    pub from: Color,
    pub date: Color,
}

impl Default for MessageColors {
    fn default() -> Self {
        Self {
            id: Color::Reset,
            subject: Color::Reset,
            from: Color::Reset,
            date: Color::Reset,
        }
    }
}

/// Renderable table of Microsoft Graph messages.
#[derive(Clone, Debug, Serialize)]
pub struct MessagesTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    #[serde(skip)]
    colors: MessageColors,
    messages: Vec<MsgraphMessage>,
}

impl fmt::Display for MessagesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("FLAGS"),
                Cell::new("SUBJECT"),
                Cell::new("FROM"),
                Cell::new("RECEIVED"),
            ]))
            .add_rows(self.messages.iter().map(|msg| {
                let flags = if msg.is_read == Some(false) { "U" } else { "" };
                let subject = msg.subject.clone().unwrap_or_default();
                let from = msg.from.as_ref().map(recipient).unwrap_or_default();
                let received = msg.received_date_time.clone().unwrap_or_default();

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&msg.id).fg(self.colors.id))
                    .add_cell(Cell::new(flags))
                    .add_cell(Cell::new(subject).fg(self.colors.subject))
                    .add_cell(Cell::new(from).fg(self.colors.from))
                    .add_cell(Cell::new(received).fg(self.colors.date));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
