use std::{fmt, io::Read};

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::messages::{
    GmailMessage, GmailMessageFormat, GmailMessageId, batch_delete::GmailMessageBatchDelete,
    batch_modify::GmailMessageBatchModify, decode_raw, encode_raw, import::GmailMessageImport,
    insert::GmailMessageInsert, list::GmailMessagesListParams,
};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::client::GmailClient,
    shared::output::{Paginated, write_bytes_or_save},
};

/// Manage Gmail messages (users.messages).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailMessagesCommand {
    List(GmailMessagesListCommand),
    Get(GmailMessageGetCommand),
    Send(GmailMessageSendCommand),
    Import(GmailMessageImportCommand),
    Insert(GmailMessageInsertCommand),
    Modify(GmailMessageModifyCommand),
    Trash(GmailMessageTrashCommand),
    Untrash(GmailMessageUntrashCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailMessageDeleteCommand),
    BatchModify(GmailMessageBatchModifyCommand),
    BatchDelete(GmailMessageBatchDeleteCommand),
}

impl GmailMessagesCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Send(cmd) => cmd.execute(printer, client),
            Self::Import(cmd) => cmd.execute(printer, client),
            Self::Insert(cmd) => cmd.execute(printer, client),
            Self::Modify(cmd) => cmd.execute(printer, client),
            Self::Trash(cmd) => cmd.execute(printer, client),
            Self::Untrash(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::BatchModify(cmd) => cmd.execute(printer, client),
            Self::BatchDelete(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List Gmail message ids matching the given query and labels
/// (users.messages.list).
#[derive(Debug, Parser)]
pub struct GmailMessagesListCommand {
    /// Gmail search query, using the same syntax as the Gmail search
    /// box (e.g. `from:alice is:unread`).
    #[arg(short = 'q', long)]
    pub query: Option<String>,

    /// Only return messages carrying the given label id. Can be
    /// repeated to require multiple labels.
    #[arg(short = 'l', long = "label", value_name = "ID")]
    pub labels: Vec<String>,

    /// Maximum number of message ids to return.
    #[arg(short = 's', long, value_name = "N")]
    pub max_results: Option<u32>,

    /// Page token returned by a previous listing, to fetch the next
    /// page.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<String>,

    /// Also include messages from SPAM and TRASH.
    #[arg(long)]
    pub include_spam_trash: bool,
}

impl GmailMessagesListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let params = GmailMessagesListParams {
            q: self.query.as_deref(),
            label_ids: &self.labels,
            max_results: self.max_results,
            page_token: self.page_token.as_deref(),
            include_spam_trash: self.include_spam_trash,
        };
        let response = client.messages_list(&params)?.response;

        let next_page = response.next_page_token;
        let table = MessageIdsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            id_color: account.envelopes_list_table_id_color(),
            ids: response.messages,
        };

        printer.out(Paginated::new(table, next_page))
    }
}

/// Get a single Gmail message (users.messages.get).
#[derive(Debug, Parser)]
pub struct GmailMessageGetCommand {
    /// The id of the message to get.
    #[arg(value_name = "ID")]
    pub id: String,

    /// The amount of message detail to return.
    #[arg(long, value_enum, default_value_t)]
    pub format: FormatArg,

    /// Header to include when `--format metadata` is used. Can be
    /// repeated.
    #[arg(long = "header", value_name = "NAME")]
    pub headers: Vec<String>,
}

impl GmailMessageGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let format = GmailMessageFormat::from(self.format);
        let hs: Vec<&str> = self.headers.iter().map(String::as_str).collect();

        let msg = client.message_get(&self.id, format, &hs)?.response;

        if format == GmailMessageFormat::Raw {
            if let Some(raw) = &msg.raw {
                let bytes =
                    decode_raw(raw).map_err(|err| anyhow!("Decode Gmail message error: {err}"))?;
                return write_bytes_or_save(printer, None, &bytes);
            }
        }

        let mut out = String::new();
        out.push_str(&format!("Id: {}\n", msg.id));
        if let Some(thread_id) = &msg.thread_id {
            out.push_str(&format!("Thread: {thread_id}\n"));
        }
        if !msg.label_ids.is_empty() {
            out.push_str(&format!("Labels: {}\n", msg.label_ids.join(", ")));
        }
        if let Some(snippet) = &msg.snippet {
            out.push_str(&format!("Snippet: {snippet}\n"));
        }
        if let Some(size) = msg.size_estimate {
            out.push_str(&format!("Size: {size}\n"));
        }
        if let Some(internal_date) = &msg.internal_date {
            out.push_str(&format!("Internal date: {internal_date}\n"));
        }

        printer.out(Message::new(out))
    }
}

/// Send a Gmail message (users.messages.send).
#[derive(Debug, Parser)]
pub struct GmailMessageSendCommand {
    /// The raw RFC 5322 message to send. Read from standard input when
    /// omitted.
    #[arg(value_name = "MESSAGE")]
    pub message: Option<String>,
}

impl GmailMessageSendCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = read_message(self.message)?;
        let message = GmailMessage {
            raw: Some(encode_raw(&raw)),
            ..Default::default()
        };
        let id = client.message_send(&message)?.response;
        printer.out(Message::new(format!(
            "Gmail message `{}` successfully sent",
            id.id
        )))
    }
}

/// Import a Gmail message into the mailbox (users.messages.import).
#[derive(Debug, Parser)]
pub struct GmailMessageImportCommand {
    /// Label id to apply to the imported message. Can be repeated.
    #[arg(long = "label", value_name = "ID")]
    pub labels: Vec<String>,

    /// The raw RFC 5322 message to import. Read from standard input
    /// when omitted.
    #[arg(value_name = "MESSAGE")]
    pub message: Option<String>,
}

impl GmailMessageImportCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = read_message(self.message)?;

        let message = GmailMessage {
            raw: Some(encode_raw(&raw)),
            label_ids: self.labels.clone(),
            ..Default::default()
        };

        let out = {
            let c = GmailMessageImport::new(
                &client.auth,
                &client.user_id,
                &message,
                None,
                false,
                false,
                false,
            )?;
            client.run(c)?
        };
        let message = out.response;

        printer.out(Message::new(format!(
            "Gmail message `{}` successfully imported",
            message.id
        )))
    }
}

/// Insert a Gmail message into the mailbox without sending
/// (users.messages.insert).
#[derive(Debug, Parser)]
pub struct GmailMessageInsertCommand {
    /// Label id to apply to the inserted message. Can be repeated.
    #[arg(long = "label", value_name = "ID")]
    pub labels: Vec<String>,

    /// The raw RFC 5322 message to insert. Read from standard input
    /// when omitted.
    #[arg(value_name = "MESSAGE")]
    pub message: Option<String>,
}

impl GmailMessageInsertCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = read_message(self.message)?;

        let message = GmailMessage {
            raw: Some(encode_raw(&raw)),
            label_ids: self.labels.clone(),
            ..Default::default()
        };

        let out = {
            let c = GmailMessageInsert::new(&client.auth, &client.user_id, &message, None, false)?;
            client.run(c)?
        };
        let message = out.response;

        printer.out(Message::new(format!(
            "Gmail message `{}` successfully inserted",
            message.id
        )))
    }
}

/// Modify the labels of a Gmail message (users.messages.modify).
#[derive(Debug, Parser)]
pub struct GmailMessageModifyCommand {
    /// The id of the message to modify.
    #[arg(value_name = "ID")]
    pub id: String,

    /// Label id to add to the message. Can be repeated.
    #[arg(long = "add-label", value_name = "ID")]
    pub add: Vec<String>,

    /// Label id to remove from the message. Can be repeated.
    #[arg(long = "remove-label", value_name = "ID")]
    pub remove: Vec<String>,
}

impl GmailMessageModifyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let message = client
            .message_modify(&self.id, &self.add, &self.remove)?
            .response;
        printer.out(Message::new(format!(
            "Gmail message `{}` successfully modified",
            message.id
        )))
    }
}

/// Move a Gmail message to the trash (users.messages.trash).
#[derive(Debug, Parser)]
pub struct GmailMessageTrashCommand {
    /// The id of the message to trash.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailMessageTrashCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let message = client.message_trash(&self.id)?.response;
        printer.out(Message::new(format!(
            "Gmail message `{}` successfully trashed",
            message.id
        )))
    }
}

/// Remove a Gmail message from the trash (users.messages.untrash).
#[derive(Debug, Parser)]
pub struct GmailMessageUntrashCommand {
    /// The id of the message to untrash.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailMessageUntrashCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let message = client.message_untrash(&self.id)?.response;
        printer.out(Message::new(format!(
            "Gmail message `{}` successfully untrashed",
            message.id
        )))
    }
}

/// Permanently delete a Gmail message (users.messages.delete).
#[derive(Debug, Parser)]
pub struct GmailMessageDeleteCommand {
    /// The id of the message to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailMessageDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        client.message_delete(&self.id)?;
        printer.out(Message::new(format!(
            "Gmail message `{}` permanently deleted",
            self.id
        )))
    }
}

/// Modify the labels of several Gmail messages at once
/// (users.messages.batchModify).
#[derive(Debug, Parser)]
pub struct GmailMessageBatchModifyCommand {
    /// The ids of the messages to modify.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,

    /// Label id to add to every message. Can be repeated.
    #[arg(long = "add-label", value_name = "ID")]
    pub add: Vec<String>,

    /// Label id to remove from every message. Can be repeated.
    #[arg(long = "remove-label", value_name = "ID")]
    pub remove: Vec<String>,
}

impl GmailMessageBatchModifyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let count = self.ids.len();

        {
            let c = GmailMessageBatchModify::new(
                &client.auth,
                &client.user_id,
                &self.ids,
                &self.add,
                &self.remove,
            )?;
            client.run(c)?
        };

        printer.out(Message::new(format!("{count} messages modified")))
    }
}

/// Permanently delete several Gmail messages at once
/// (users.messages.batchDelete).
#[derive(Debug, Parser)]
pub struct GmailMessageBatchDeleteCommand {
    /// The ids of the messages to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl GmailMessageBatchDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let count = self.ids.len();

        {
            let c = GmailMessageBatchDelete::new(&client.auth, &client.user_id, &self.ids)?;
            client.run(c)?
        };

        printer.out(Message::new(format!(
            "{count} messages permanently deleted"
        )))
    }
}

/// Gmail message format requested by `messages get`.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FormatArg {
    Minimal,
    #[default]
    Full,
    Raw,
    Metadata,
}

impl From<FormatArg> for GmailMessageFormat {
    fn from(arg: FormatArg) -> Self {
        match arg {
            FormatArg::Minimal => GmailMessageFormat::Minimal,
            FormatArg::Full => GmailMessageFormat::Full,
            FormatArg::Raw => GmailMessageFormat::Raw,
            FormatArg::Metadata => GmailMessageFormat::Metadata,
        }
    }
}

/// Renders a list of Gmail message ids as a two-column table.
#[derive(Clone, Debug, Serialize)]
pub struct MessageIdsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    #[serde(skip)]
    pub id_color: Color,
    pub ids: Vec<GmailMessageId>,
}

impl fmt::Display for MessageIdsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("ID"), Cell::new("THREAD ID")]))
            .add_rows(self.ids.iter().map(|id| {
                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&id.id).fg(self.id_color))
                    .add_cell(Cell::new(id.thread_id.as_deref().unwrap_or("")));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

/// Reads a raw RFC 5322 message either from the given argument or, when
/// absent, from standard input.
fn read_message(arg: Option<String>) -> Result<Vec<u8>> {
    match arg {
        Some(message) => Ok(message.into_bytes()),
        None => {
            let mut raw = String::new();
            std::io::stdin().read_to_string(&mut raw)?;
            Ok(raw.into_bytes())
        }
    }
}
