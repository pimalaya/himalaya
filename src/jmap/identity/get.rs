//! # JMAP identity get
//!
//! The `jmap identity get` command, RFC 8621 `Identity/get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::identity::{JmapIdentity, get::JmapIdentityGetOptions};
use log::warn;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, backend::Backend, jmap::client::JmapClient, listing::Listing,
};

/// Get JMAP identities (Identity/get).
///
/// Lists sender identities available for sending email. Pass no IDs to
/// list all identities.
#[derive(Debug, Parser)]
pub struct JmapIdentityGetCommand {
    /// Identity ID(s) to retrieve (omit to get all).
    #[arg(value_name = "ID")]
    pub ids: Option<Vec<String>>,
}

impl JmapIdentityGetCommand {
    /// Fetches the identities and lists them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = client.identity_get(JmapIdentityGetOptions { ids: self.ids })?;

        for id in output.not_found {
            warn!("identity `{id}` not found");
        }

        printer.out(JmapIdentityGetOutput {
            account: account.name().to_string(),
            identities: output.identities,
        })
    }
}

/// The `jmap identity get` output, the identities the server holds.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JmapIdentityGetOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The identities, in the order the server returned them.
    pub identities: Vec<JmapIdentity>,
}

impl fmt::Display for JmapIdentityGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Jmap));

        for identity in &self.identities {
            listing.items.push([
                sanitize::text(&identity.id),
                sanitize::text(&identity.email),
                sanitize::text(&identity.name),
            ]);
        }

        write!(f, "{listing}")
    }
}
