//! # JMAP thread get
//!
//! The `jmap thread get` command, RFC 8621 `Thread/get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::thread::JmapThread;
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    error::{ErrorCode, ItemError, partial_when},
    jmap::{backend::not_found_errors, client::JmapClient},
};

/// Get JMAP threads by ID (Thread/get).
///
/// Each thread contains an ordered list of email IDs in the thread. An id
/// the server does not find is an entry of `errors`, coded
/// `thread-not-found`, and exits 3.
#[derive(Debug, Parser)]
pub struct JmapThreadGetCommand {
    /// Thread ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapThreadGetCommand {
    /// Fetches the threads and lists the emails they hold, then the error
    /// of each id not found.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let output = client.thread_get(self.ids.clone())?;

        let threads = JmapThreadGetOutput {
            account: account.name().to_string(),
            threads: output.threads,
            errors: not_found_errors(output.not_found, ErrorCode::ThreadNotFound),
        };

        printer.out(&threads)?;
        partial_when(&threads.errors)
    }
}

/// The `jmap thread get` output, the threads with the emails they hold.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JmapThreadGetOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The threads, in the order the server returned them.
    pub threads: Vec<JmapThread>,
    /// The error of each thread the server did not find, in the order
    /// met.
    pub errors: Vec<ItemError>,
}

impl fmt::Display for JmapThreadGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: one row per email, in thread order, so each line is one
        // pair a script can read, and an empty thread one row of its own.
        let mut listing = Listing::new(&self.account, Some(Backend::Jmap.as_str()));

        for thread in &self.threads {
            let id = sanitize::text(&thread.id);

            if thread.email_ids.is_empty() {
                listing.items.push([&*id, ""]);
            }

            for email_id in &thread.email_ids {
                listing.items.push([&*id, &*sanitize::text(email_id)]);
            }
        }

        for error in &self.errors {
            listing.errors.push(error);
        }

        write!(f, "{listing}")
    }
}
