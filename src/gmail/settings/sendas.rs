use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::settings::send_as::{
    GmailSendAs, create::GmailSendAsCreate, delete::GmailSendAsDelete, get::GmailSendAsGet,
    list::GmailSendAsList, patch::GmailSendAsPatch, update::GmailSendAsUpdate,
    verify::GmailSendAsVerify,
};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
};

/// Manage Gmail send-as aliases (settings.sendAs).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsSendAsCommand {
    List(GmailSendAsListCommand),
    Get(GmailSendAsGetCommand),
    Create(GmailSendAsCreateCommand),
    Update(GmailSendAsUpdateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailSendAsDeleteCommand),
    Verify(GmailSendAsVerifyCommand),
}

impl GmailSettingsSendAsCommand {
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
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::Verify(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List all Gmail send-as aliases (settings.sendAs.list).
#[derive(Debug, Parser)]
pub struct GmailSendAsListCommand;

impl GmailSendAsListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let c = GmailSendAsList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };

        let table = SendAsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            send_as: out.response.send_as,
        };

        printer.out(table)
    }
}

/// Get one Gmail send-as alias by e-mail address (settings.sendAs.get).
#[derive(Debug, Parser)]
pub struct GmailSendAsGetCommand {
    /// E-mail address of the send-as alias to get.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSendAsGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailSendAsGet::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?
        };
        let send_as = out.response;

        let mut buf = String::new();
        buf.push_str(&format!("Email: {}\n", send_as.send_as_email));
        if let Some(display_name) = send_as.display_name {
            buf.push_str(&format!("Name: {display_name}\n"));
        }
        if let Some(reply_to_address) = send_as.reply_to_address {
            buf.push_str(&format!("Reply-To: {reply_to_address}\n"));
        }
        if let Some(signature) = send_as.signature {
            buf.push_str(&format!("Signature: {signature}\n"));
        }
        if let Some(is_primary) = send_as.is_primary {
            buf.push_str(&format!("Primary: {is_primary}\n"));
        }
        if let Some(is_default) = send_as.is_default {
            buf.push_str(&format!("Default: {is_default}\n"));
        }
        if let Some(treat_as_alias) = send_as.treat_as_alias {
            buf.push_str(&format!("Treat as alias: {treat_as_alias}\n"));
        }
        if let Some(verification_status) = send_as.verification_status {
            buf.push_str(&format!(
                "Verification: {}\n",
                verification_status_wire(verification_status)
            ));
        }

        printer.out(Message::new(buf))
    }
}

/// Create a Gmail send-as alias (settings.sendAs.create).
#[derive(Debug, Parser)]
pub struct GmailSendAsCreateCommand {
    /// E-mail address of the send-as alias to create.
    #[arg(value_name = "EMAIL")]
    pub email: String,

    /// Display name shown in the From header for this alias.
    #[arg(long)]
    pub display_name: Option<String>,

    /// Reply-To address to set on messages sent from this alias.
    #[arg(long)]
    pub reply_to_address: Option<String>,

    /// HTML signature appended to messages sent from this alias.
    #[arg(long)]
    pub signature: Option<String>,

    /// Treat this alias as an alias of the primary address.
    #[arg(long)]
    pub treat_as_alias: bool,
}

impl GmailSendAsCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let send_as = GmailSendAs {
            send_as_email: self.email.clone(),
            display_name: self.display_name,
            reply_to_address: self.reply_to_address,
            signature: self.signature,
            treat_as_alias: self.treat_as_alias.then_some(true),
            ..Default::default()
        };

        let out = {
            let c = GmailSendAsCreate::new(&client.auth, &client.user_id, &send_as)?;
            client.run(c)?
        };
        let created = out.response;

        printer.out(Message::new(format!(
            "Gmail send-as `{}` successfully created",
            created.send_as_email
        )))
    }
}

/// Update a Gmail send-as alias (settings.sendAs.update/patch).
#[derive(Debug, Parser)]
pub struct GmailSendAsUpdateCommand {
    /// E-mail address of the send-as alias to update.
    #[arg(value_name = "EMAIL")]
    pub email: String,

    /// Display name shown in the From header for this alias.
    #[arg(long)]
    pub display_name: Option<String>,

    /// Reply-To address to set on messages sent from this alias.
    #[arg(long)]
    pub reply_to_address: Option<String>,

    /// HTML signature appended to messages sent from this alias.
    #[arg(long)]
    pub signature: Option<String>,

    /// Treat this alias as an alias of the primary address.
    #[arg(long)]
    pub treat_as_alias: bool,

    /// Switch from a full update to a partial patch; without it the
    /// default update clears any field you omit.
    #[arg(long)]
    pub patch: bool,
}

impl GmailSendAsUpdateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let send_as = GmailSendAs {
            send_as_email: self.email.clone(),
            display_name: self.display_name,
            reply_to_address: self.reply_to_address,
            signature: self.signature,
            treat_as_alias: self.treat_as_alias.then_some(true),
            ..Default::default()
        };

        if self.patch {
            let c = GmailSendAsPatch::new(&client.auth, &client.user_id, &self.email, &send_as)?;
            client.run(c)?;
        } else {
            let c = GmailSendAsUpdate::new(&client.auth, &client.user_id, &self.email, &send_as)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Gmail send-as `{}` successfully updated",
            self.email
        )))
    }
}

/// Delete a Gmail send-as alias (settings.sendAs.delete).
#[derive(Debug, Parser)]
pub struct GmailSendAsDeleteCommand {
    /// E-mail address of the send-as alias to delete.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSendAsDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailSendAsDelete::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Gmail send-as `{}` successfully deleted",
            self.email
        )))
    }
}

/// Send a verification e-mail for a Gmail send-as alias
/// (settings.sendAs.verify).
#[derive(Debug, Parser)]
pub struct GmailSendAsVerifyCommand {
    /// E-mail address of the send-as alias to verify.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSendAsVerifyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailSendAsVerify::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Verification e-mail sent for Gmail send-as `{}`",
            self.email
        )))
    }
}

/// Renderable table of Gmail send-as aliases.
#[derive(Clone, Debug, Serialize)]
pub struct SendAsTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    send_as: Vec<GmailSendAs>,
}

impl fmt::Display for SendAsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("EMAIL"),
                Cell::new("NAME"),
                Cell::new("DEFAULT"),
                Cell::new("VERIFICATION"),
            ]))
            .add_rows(self.send_as.iter().map(|send_as| {
                let default = if send_as.is_default == Some(true) {
                    "yes"
                } else {
                    ""
                };

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&send_as.send_as_email).fg(Color::Reset))
                    .add_cell(
                        Cell::new(send_as.display_name.as_deref().unwrap_or("")).fg(Color::Reset),
                    )
                    .add_cell(Cell::new(default).fg(Color::Reset))
                    .add_cell(
                        Cell::new(
                            send_as
                                .verification_status
                                .map(verification_status_wire)
                                .unwrap_or_default(),
                        )
                        .fg(Color::Reset),
                    );
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
