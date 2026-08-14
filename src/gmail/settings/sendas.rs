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
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
    shared::table::style_from_preset,
};

/// Manage Gmail send-as aliases (settings.sendAs).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsSendAsCommand {
    List(GmailSettingsSendAsListCommand),
    Get(GmailSettingsSendAsGetCommand),
    Create(GmailSettingsSendAsCreateCommand),
    Update(GmailSettingsSendAsUpdateCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailSettingsSendAsDeleteCommand),
    Verify(GmailSettingsSendAsVerifyCommand),
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
pub struct GmailSettingsSendAsListCommand;

impl GmailSettingsSendAsListCommand {
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
pub struct GmailSettingsSendAsGetCommand {
    /// E-mail address of the send-as alias to get.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsSendAsGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailSendAsGet::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?
        };
        let send_as = out.response;

        printer.out(GmailSettingsSendAsGetOutput(send_as))
    }
}

/// Create a Gmail send-as alias (settings.sendAs.create).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsCreateCommand {
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

impl GmailSettingsSendAsCreateCommand {
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
pub struct GmailSettingsSendAsUpdateCommand {
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

impl GmailSettingsSendAsUpdateCommand {
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
pub struct GmailSettingsSendAsDeleteCommand {
    /// E-mail address of the send-as alias to delete.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsSendAsDeleteCommand {
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
pub struct GmailSettingsSendAsVerifyCommand {
    /// E-mail address of the send-as alias to verify.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsSendAsVerifyCommand {
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

/// A Gmail send-as alias, rendered as aligned text or, under `--json`,
/// as the send-as resource itself instead of a wrapped human string.
///
/// The resource is emitted verbatim so that one alias read with `get`
/// has the very same shape as a row of `list`.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsSendAsGetOutput(GmailSendAs);

impl fmt::Display for GmailSettingsSendAsGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Email: {}", self.0.send_as_email)?;

        if let Some(display_name) = &self.0.display_name {
            writeln!(f, "Name: {display_name}")?;
        }
        if let Some(reply_to_address) = &self.0.reply_to_address {
            writeln!(f, "Reply-To: {reply_to_address}")?;
        }
        if let Some(signature) = &self.0.signature {
            writeln!(f, "Signature: {signature}")?;
        }
        if let Some(is_primary) = self.0.is_primary {
            writeln!(f, "Primary: {is_primary}")?;
        }
        if let Some(is_default) = self.0.is_default {
            writeln!(f, "Default: {is_default}")?;
        }
        if let Some(treat_as_alias) = self.0.treat_as_alias {
            writeln!(f, "Treat as alias: {treat_as_alias}")?;
        }
        if let Some(status) = self.0.verification_status {
            writeln!(f, "Verification: {}", verification_status_wire(status))?;
        }

        Ok(())
    }
}

/// Renderable table of Gmail send-as aliases.
#[derive(Clone, Debug, Serialize, JsonSchema)]
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
            .load_style(style_from_preset(&self.preset))
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
