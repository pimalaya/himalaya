use std::{fmt, io::Read};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_msgraph::v1::rest::users::messages::{
    MsgraphImportance, MsgraphMessage, MsgraphRecipient, list::MsgraphMessagesListParams,
};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

use crate::{account::context::Account, msgraph::client::MsgraphClient};

/// Manage Microsoft Graph messages (`me.messages`).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MsgraphMessagesCommand {
    List(MsgraphMessagesListCommand),
    Get(MsgraphMessageGetCommand),
    Create(MsgraphMessageCreateCommand),
    Update(MsgraphMessageUpdateCommand),
    Send(MsgraphMessageSendCommand),
    Copy(MsgraphMessageCopyCommand),
    #[command(name = "move")]
    Move(MsgraphMessageMoveCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(MsgraphMessageDeleteCommand),
}

impl MsgraphMessagesCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Send(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List Microsoft Graph messages (`GET /me/messages` or, with
/// `--folder`, `GET /me/mailFolders/{id}/messages`).
#[derive(Debug, Parser)]
pub struct MsgraphMessagesListCommand {
    /// Restrict the listing to a folder id or well-known name (e.g.
    /// `inbox`). Lists the whole mailbox when omitted.
    #[arg(short = 'f', long, value_name = "ID")]
    pub folder: Option<String>,

    /// Maximum number of messages to return (OData `$top`).
    #[arg(short = 's', long, value_name = "N")]
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

impl MsgraphMessagesListCommand {
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

        if let Some(link) = response.next_link {
            log::info!("next page link: {link}");
        }

        printer.out(messages_table(account, response.value))
    }
}

/// Get a single Microsoft Graph message (`GET /me/messages/{id}`), or its
/// raw RFC 5322 bytes with `--raw` (`GET /me/messages/{id}/$value`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageGetCommand {
    /// The id of the message to get.
    #[arg(value_name = "ID")]
    pub id: String,

    /// Return the raw RFC 5322 MIME message instead of the parsed
    /// fields.
    #[arg(long)]
    pub raw: bool,
}

impl MsgraphMessageGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        if self.raw {
            let bytes = client.message_get_raw(&self.id)?.response;
            let content = String::from_utf8_lossy(&bytes).into_owned();
            return printer.out(Message::new(content));
        }

        let msg = client.message_get(&self.id)?.response;

        let mut out = String::new();
        out.push_str(&format!("Id: {}\n", msg.id));
        if let Some(subject) = &msg.subject {
            out.push_str(&format!("Subject: {subject}\n"));
        }
        if let Some(from) = msg.from.as_ref().map(recipient) {
            out.push_str(&format!("From: {from}\n"));
        }
        let to: Vec<String> = msg.to_recipients.iter().map(recipient).collect();
        if !to.is_empty() {
            out.push_str(&format!("To: {}\n", to.join(", ")));
        }
        if let Some(date) = &msg.received_date_time {
            out.push_str(&format!("Received: {date}\n"));
        }
        if let Some(is_read) = msg.is_read {
            out.push_str(&format!("Read: {is_read}\n"));
        }
        if let Some(folder) = &msg.parent_folder_id {
            out.push_str(&format!("Folder: {folder}\n"));
        }
        if let Some(preview) = &msg.body_preview {
            out.push_str(&format!("Preview: {preview}\n"));
        }

        printer.out(Message::new(out))
    }
}

/// Create a Microsoft Graph draft message from raw MIME (`POST
/// /me/messages`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageCreateCommand {
    /// Create the draft in this folder id or well-known name (e.g.
    /// `drafts`). Defaults to the mailbox root.
    #[arg(short = 'f', long, value_name = "ID")]
    pub folder: Option<String>,

    /// The raw RFC 5322 message to store as a draft. Read from standard
    /// input when omitted.
    #[arg(value_name = "MESSAGE")]
    pub message: Option<String>,
}

impl MsgraphMessageCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let raw = read_message(self.message)?;
        let message = client
            .message_create_mime(self.folder.as_deref(), &raw)?
            .response;
        printer.out(Message::new(format!(
            "Microsoft Graph draft `{}` successfully created",
            message.id
        )))
    }
}

/// Update a Microsoft Graph message (`PATCH /me/messages/{id}`): mark
/// read/unread, set importance or replace categories.
#[derive(Debug, Parser)]
pub struct MsgraphMessageUpdateCommand {
    /// The id of the message to update.
    #[arg(value_name = "ID")]
    pub id: String,

    /// Mark the message as read.
    #[arg(long, conflicts_with = "unread")]
    pub read: bool,

    /// Mark the message as unread.
    #[arg(long)]
    pub unread: bool,

    /// Set the message importance.
    #[arg(long, value_enum, value_name = "LEVEL")]
    pub importance: Option<ImportanceArg>,

    /// Category to set on the message. Can be repeated; replaces the
    /// existing categories.
    #[arg(long = "category", value_name = "NAME")]
    pub categories: Vec<String>,
}

impl MsgraphMessageUpdateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let mut patch = MsgraphMessage::default();
        if self.read {
            patch.is_read = Some(true);
        }
        if self.unread {
            patch.is_read = Some(false);
        }
        if let Some(importance) = self.importance {
            patch.importance = Some(importance.into());
        }
        patch.categories = self.categories;

        let message = client.message_update(&self.id, &patch)?.response;
        printer.out(Message::new(format!(
            "Microsoft Graph message `{}` successfully updated",
            message.id
        )))
    }
}

/// Send a Microsoft Graph message from raw MIME (`POST /me/sendMail`);
/// Graph saves it to Sent Items.
#[derive(Debug, Parser)]
pub struct MsgraphMessageSendCommand {
    /// The raw RFC 5322 message to send. Read from standard input when
    /// omitted.
    #[arg(value_name = "MESSAGE")]
    pub message: Option<String>,
}

impl MsgraphMessageSendCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let raw = read_message(self.message)?;
        client.send_mail_mime(&raw)?;
        printer.out(Message::new("Microsoft Graph message successfully sent"))
    }
}

/// Copy a Microsoft Graph message into another folder (`POST
/// /me/messages/{id}/copy`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageCopyCommand {
    /// The id of the message to copy.
    #[arg(value_name = "ID")]
    pub id: String,

    /// The destination folder id or well-known name.
    #[arg(value_name = "DESTINATION")]
    pub destination: String,
}

impl MsgraphMessageCopyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let message = client.message_copy(&self.id, &self.destination)?.response;
        printer.out(Message::new(format!(
            "Microsoft Graph message copied to `{}`",
            message.id
        )))
    }
}

/// Move a Microsoft Graph message into another folder (`POST
/// /me/messages/{id}/move`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageMoveCommand {
    /// The id of the message to move.
    #[arg(value_name = "ID")]
    pub id: String,

    /// The destination folder id or well-known name.
    #[arg(value_name = "DESTINATION")]
    pub destination: String,
}

impl MsgraphMessageMoveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let message = client.message_move(&self.id, &self.destination)?.response;
        printer.out(Message::new(format!(
            "Microsoft Graph message moved to `{}`",
            message.id
        )))
    }
}

/// Permanently delete a Microsoft Graph message (`DELETE
/// /me/messages/{id}`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageDeleteCommand {
    /// The id of the message to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl MsgraphMessageDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        client.message_delete(&self.id)?;
        printer.out(Message::new(format!(
            "Microsoft Graph message `{}` permanently deleted",
            self.id
        )))
    }
}

/// Message importance requested by `messages update`.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum ImportanceArg {
    Low,
    Normal,
    High,
}

impl From<ImportanceArg> for MsgraphImportance {
    fn from(arg: ImportanceArg) -> Self {
        match arg {
            ImportanceArg::Low => MsgraphImportance::Low,
            ImportanceArg::Normal => MsgraphImportance::Normal,
            ImportanceArg::High => MsgraphImportance::High,
        }
    }
}

/// Renders the display form of a recipient: `Name <addr>`, or just the
/// address when there is no display name.
fn recipient(recipient: &MsgraphRecipient) -> String {
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
