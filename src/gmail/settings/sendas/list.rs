//! # Gmail send-as list
//!
//! The `gmail settings sendas list` command,
//! `users.settings.sendAs.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::settings::send_as::{GmailSendAs, list::GmailSendAsList};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::verification_status_wire},
    shared::table::style_from_preset,
};

/// List all Gmail send-as aliases (settings.sendAs.list).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsListCommand;

impl GmailSettingsSendAsListCommand {
    /// Lists every send-as alias and tables it.
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
