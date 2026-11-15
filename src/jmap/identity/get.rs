//! # JMAP identity get
//!
//! The `jmap identity get` command, RFC 8621 `Identity/get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    backend::Backend,
    error::{ErrorCode, ItemError, partial_when},
    jmap::{backend::not_found_errors, client::JmapClient, raw},
};

/// Get JMAP identities (Identity/get).
///
/// Lists sender identities available for sending email. Pass no IDs to
/// list all identities. An id the server does not find is an entry of
/// `errors`, coded `identity-not-found`, and exits 3.
///
/// JSON output: `{"identities": [<JMAP identity>...], "state", "errors"}`,
/// each identity and the get's `state` as the server sent them.
#[derive(Debug, Parser)]
pub struct JmapIdentityGetCommand {
    /// Identity ID(s) to retrieve (omit to get all).
    #[arg(value_name = "ID")]
    pub ids: Option<Vec<String>>,
}

impl JmapIdentityGetCommand {
    /// Fetches the identities and lists them, then the error of each id
    /// not found.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = raw::get(client, "Identity", self.ids.as_deref())?;

        let identities = JmapIdentityGetOutput {
            account: account.name().to_string(),
            identities: output.list,
            rest: output.rest,
            errors: not_found_errors(output.not_found, ErrorCode::IdentityNotFound),
        };

        printer.out(&identities)?;
        partial_when(&identities.errors)
    }
}

/// The `jmap identity get` output, the identities the server holds.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JmapIdentityGetOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The identities, each as the server sent it, in its order.
    #[schemars(with = "Vec<Map<String, Value>>")]
    pub identities: Vec<Value>,
    /// The rest of the server's answer: the get's `state`.
    #[serde(flatten)]
    pub rest: Map<String, Value>,
    /// The error of each identity the server did not find, in the order
    /// met.
    pub errors: Vec<ItemError>,
}

impl fmt::Display for JmapIdentityGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Jmap.as_str()));

        for identity in &self.identities {
            let text = |key| {
                sanitize::text(
                    identity
                        .get(key)
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
            };
            listing
                .items
                .push([text("id"), text("email"), text("name")]);
        }

        for error in &self.errors {
            listing.errors.push(error);
        }

        write!(f, "{listing}")
    }
}
