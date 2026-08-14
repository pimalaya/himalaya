use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::settings::delegates::{
    GmailDelegate,
    create::GmailDelegateCreate,
    delete::GmailDelegateDelete,
    get::GmailDelegateGet,
    list::{GmailDelegatesList, GmailDelegatesListResponse},
};
use pimalaya_cli::printer::{Message, Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
    shared::table::style_from_preset,
};

/// Manage Gmail delegates (users.settings.delegates).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsDelegatesCommand {
    List(List),
    Get(Get),
    Create(Create),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(Delete),
}

impl GmailSettingsDelegatesCommand {
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
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List all Gmail delegates (users.settings.delegates.list).
#[derive(Debug, Parser)]
pub struct List;

impl List {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let c = GmailDelegatesList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let resp = out.response;

        let table = DelegatesTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            response: resp,
        };

        printer.out(table)
    }
}

/// Get a Gmail delegate by email address (users.settings.delegates.get).
#[derive(Debug, Parser)]
pub struct Get {
    /// Email address of the delegate to get.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl Get {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailDelegateGet::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?
        };
        let delegate = out.response;

        printer.out(GmailSettingsDelegateGetOutput(delegate))
    }
}

/// Create a Gmail delegate (users.settings.delegates.create).
#[derive(Debug, Parser)]
pub struct Create {
    /// Email address of the delegate to create.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl Create {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let delegate = GmailDelegate {
            delegate_email: self.email.clone(),
            verification_status: None,
        };

        let out = {
            let c = GmailDelegateCreate::new(&client.auth, &client.user_id, &delegate)?;
            client.run(c)?
        };
        let created = out.response;

        printer.out(Message::new(format!(
            "Gmail delegate `{}` successfully created",
            created.delegate_email
        )))
    }
}

/// Delete a Gmail delegate (users.settings.delegates.delete).
#[derive(Debug, Parser)]
pub struct Delete {
    /// Email address of the delegate to delete.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl Delete {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailDelegateDelete::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Gmail delegate `{}` successfully deleted",
            self.email
        )))
    }
}

/// A Gmail delegate, rendered as aligned text or, under `--json`, as
/// the delegate resource itself instead of a wrapped human string.
///
/// The resource is emitted verbatim so that one delegate read with `get`
/// has the very same shape as a row of `list`.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsDelegateGetOutput(GmailDelegate);

impl fmt::Display for GmailSettingsDelegateGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Email: {}", self.0.delegate_email)?;

        if let Some(status) = self.0.verification_status {
            writeln!(f, "Verification: {}", verification_status_wire(status))?;
        }

        Ok(())
    }
}

/// Renderable table of Gmail delegates.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct DelegatesTable {
    #[serde(skip)]
    #[schemars(skip)]
    preset: String,
    #[serde(skip)]
    #[schemars(skip)]
    arrangement: ContentArrangement,
    response: GmailDelegatesListResponse,
}

impl fmt::Display for DelegatesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("EMAIL"), Cell::new("VERIFICATION")]))
            .add_rows(self.response.delegates.iter().map(|delegate| {
                let status = delegate
                    .verification_status
                    .map(verification_status_wire)
                    .unwrap_or_default();

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&delegate.delegate_email).fg(Color::Reset))
                    .add_cell(Cell::new(status).fg(Color::Reset));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
