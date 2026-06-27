use std::{fmt, io::Read};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::{
    drafts::{
        GmailDraft,
        create::GmailDraftCreate,
        delete::GmailDraftDelete,
        get::GmailDraftGet,
        list::{GmailDraftsList, GmailDraftsListParams},
        send::GmailDraftSend,
        update::GmailDraftUpdate,
    },
    messages::{GmailMessage, GmailMessageFormat, encode_raw},
};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

use crate::{account::context::Account, gmail::client::GmailClient, shared::output::Paginated};

/// Manage Gmail drafts (users.drafts).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailDraftsCommand {
    List(GmailDraftsListCommand),
    Get(GmailDraftGetCommand),
    Create(GmailDraftCreateCommand),
    Update(GmailDraftUpdateCommand),
    Send(GmailDraftSendCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailDraftDeleteCommand),
}

impl GmailDraftsCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Send(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List Gmail drafts (users.drafts.list).
#[derive(Debug, Parser)]
pub struct GmailDraftsListCommand {
    /// Gmail search query, using the same syntax as the Gmail search
    /// box (e.g. `from:alice is:unread`).
    #[arg(short = 'q', long)]
    pub query: Option<String>,

    /// Maximum number of drafts to return.
    #[arg(short = 's', long, value_name = "N")]
    pub max_results: Option<u32>,

    /// Page token returned by a previous listing, to fetch the next
    /// page.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<String>,

    /// Also include drafts from SPAM and TRASH.
    #[arg(long)]
    pub include_spam_trash: bool,
}

impl GmailDraftsListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let params = GmailDraftsListParams {
                q: self.query.as_deref(),
                max_results: self.max_results,
                page_token: self.page_token.as_deref(),
                include_spam_trash: self.include_spam_trash,
            };
            let c = GmailDraftsList::new(&client.auth, &client.user_id, &params)?;
            client.run(c)?
        };
        let response = out.response;

        let next_page = response.next_page_token;
        let table = DraftsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            drafts: response.drafts,
        };

        printer.out(Paginated::new(table, next_page))
    }
}

/// Get a single Gmail draft (users.drafts.get).
#[derive(Debug, Parser)]
pub struct GmailDraftGetCommand {
    /// The id of the draft to get.
    #[arg(value_name = "ID")]
    pub id: String,

    /// The amount of message detail to return.
    #[arg(long, value_enum, default_value_t)]
    pub format: FormatArg,
}

impl GmailDraftGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let format = GmailMessageFormat::from(self.format);

        let draft = {
            let c = GmailDraftGet::new(&client.auth, &client.user_id, &self.id, format)?;
            client.run(c)?
        }
        .response;

        let mut out = String::new();
        out.push_str(&format!("Draft id: {}\n", draft.id));
        if let Some(message) = &draft.message {
            out.push_str(&format!("Message id: {}\n", message.id));
            if let Some(thread_id) = &message.thread_id {
                out.push_str(&format!("Thread: {thread_id}\n"));
            }
            if let Some(snippet) = &message.snippet {
                out.push_str(&format!("Snippet: {snippet}\n"));
            }
        }

        printer.out(Message::new(out))
    }
}

/// Create a Gmail draft (users.drafts.create).
#[derive(Debug, Parser)]
pub struct GmailDraftCreateCommand {
    /// Thread id to attach the draft to.
    #[arg(long = "thread-id", value_name = "ID")]
    pub thread_id: Option<String>,

    /// The raw RFC 5322 message to draft. Read from standard input when
    /// omitted.
    #[arg(value_name = "MESSAGE")]
    pub message: Option<String>,
}

impl GmailDraftCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = read_message(self.message)?;

        let draft = GmailDraft {
            id: String::new(),
            message: Some(GmailMessage {
                raw: Some(encode_raw(&raw)),
                thread_id: self.thread_id.clone(),
                ..Default::default()
            }),
        };

        let draft = {
            let c = GmailDraftCreate::new(&client.auth, &client.user_id, &draft)?;
            client.run(c)?
        }
        .response;

        printer.out(Message::new(format!(
            "Gmail draft `{}` successfully created",
            draft.id
        )))
    }
}

/// Update a Gmail draft (users.drafts.update).
#[derive(Debug, Parser)]
pub struct GmailDraftUpdateCommand {
    /// The id of the draft to update.
    #[arg(value_name = "ID")]
    pub id: String,

    /// Thread id to attach the draft to.
    #[arg(long = "thread-id", value_name = "ID")]
    pub thread_id: Option<String>,

    /// The raw RFC 5322 message to draft. Read from standard input when
    /// omitted.
    #[arg(value_name = "MESSAGE")]
    pub message: Option<String>,
}

impl GmailDraftUpdateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = read_message(self.message)?;

        let draft = GmailDraft {
            id: self.id.clone(),
            message: Some(GmailMessage {
                raw: Some(encode_raw(&raw)),
                thread_id: self.thread_id.clone(),
                ..Default::default()
            }),
        };

        let draft = {
            let c = GmailDraftUpdate::new(&client.auth, &client.user_id, &draft)?;
            client.run(c)?
        }
        .response;

        printer.out(Message::new(format!(
            "Gmail draft `{}` successfully updated",
            draft.id
        )))
    }
}

/// Send a Gmail draft (users.drafts.send).
#[derive(Debug, Parser)]
pub struct GmailDraftSendCommand {
    /// The id of the draft to send.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailDraftSendCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let draft = GmailDraft {
            id: self.id.clone(),
            message: None,
        };

        let message_id = {
            let c = GmailDraftSend::new(&client.auth, &client.user_id, &draft)?;
            client.run(c)?
        }
        .response;

        printer.out(Message::new(format!(
            "Gmail draft sent as message `{}`",
            message_id.id
        )))
    }
}

/// Permanently delete a Gmail draft (users.drafts.delete).
#[derive(Debug, Parser)]
pub struct GmailDraftDeleteCommand {
    /// The id of the draft to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailDraftDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailDraftDelete::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };

        printer.out(Message::new(format!(
            "Gmail draft `{}` successfully deleted",
            self.id
        )))
    }
}

/// Gmail message format requested by `drafts get`.
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

/// Renders a list of Gmail drafts as a three-column table.
#[derive(Clone, Debug, Serialize)]
pub struct DraftsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    pub drafts: Vec<GmailDraft>,
}

impl fmt::Display for DraftsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("DRAFT ID"),
                Cell::new("MESSAGE ID"),
                Cell::new("THREAD ID"),
            ]))
            .add_rows(self.drafts.iter().map(|d| {
                let message_id = d.message.as_ref().map(|m| m.id.as_str()).unwrap_or("");
                let thread_id = d
                    .message
                    .as_ref()
                    .and_then(|m| m.thread_id.as_deref())
                    .unwrap_or("");

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&d.id).fg(Color::Reset))
                    .add_cell(Cell::new(message_id).fg(Color::Reset))
                    .add_cell(Cell::new(thread_id).fg(Color::Reset));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

/// Reads a raw RFC 5322 message either from the given argument or, when
/// absent, from standard input.
fn read_message(arg: Option<String>) -> anyhow::Result<Vec<u8>> {
    match arg {
        Some(message) => Ok(message.into_bytes()),
        None => {
            let mut raw = String::new();
            std::io::stdin().read_to_string(&mut raw)?;
            Ok(raw.into_bytes())
        }
    }
}
