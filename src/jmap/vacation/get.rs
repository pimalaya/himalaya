//! # JMAP vacation get
//!
//! The `jmap vacation-response get` command, RFC 8621
//! `VacationResponse/get`.

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::vacation_response::{
    JMAP_VACATION_RESPONSE_CAPABILITY, JmapVacationResponse,
};
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, jmap::client::JmapClient, shared::message::read::sanitize_body,
};

/// Get the JMAP vacation response (VacationResponse/get).
#[derive(Debug, Parser)]
pub struct JmapVacationGetCommand;

impl JmapVacationGetCommand {
    /// Fetches the vacation response and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let has_vacation = client
            .session()
            .map(|s| {
                s.capabilities
                    .contains_key(JMAP_VACATION_RESPONSE_CAPABILITY)
            })
            .unwrap_or(false);

        if !has_vacation {
            bail!("Vacation response is not supported by the server");
        }

        // NOTE: RFC 8621 makes the singleton always exist; a server
        // returning none holds no response, which reads as a disabled one.
        let vacation = client
            .vacation_response_get()?
            .unwrap_or_else(|| JmapVacationResponse {
                id: "singleton".into(),
                is_enabled: false,
                from_date: None,
                to_date: None,
                subject: None,
                text_body: None,
                html_body: None,
            });

        printer.out(JmapVacationResponseGetOutput { vacation })
    }
}

/// The `jmap vacation get` output, the vacation response the server
/// holds.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct JmapVacationResponseGetOutput {
    /// The vacation response the server holds.
    pub vacation: JmapVacationResponse,
}

impl fmt::Display for JmapVacationResponseGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = &self.vacation;
        let mut fields = Fields::default();

        fields.push("Enabled", v.is_enabled);

        if let Some(d) = &v.from_date {
            fields.push("From", sanitize::text(d));
        }
        if let Some(d) = &v.to_date {
            fields.push("To", sanitize::text(d));
        }
        if let Some(s) = &v.subject {
            fields.push("Subject", sanitize::text(s));
        }
        if let Some(b) = &v.text_body {
            fields.push("Body (plain)", sanitize_body(b));
        }
        if let Some(b) = &v.html_body {
            fields.push("Body (HTML)", sanitize_body(b));
        }

        write!(f, "{fields}")
    }
}
