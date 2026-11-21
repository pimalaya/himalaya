//! # JMAP thread get
//!
//! The `jmap thread get` command, RFC 8621 `Thread/get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    error::{ErrorCode, ItemError, partial_when},
    jmap::{backend::not_found_errors, client::JmapClient, raw},
    unified::backend::Backend,
};

/// Get JMAP threads by ID (Thread/get).
///
/// Each thread contains an ordered list of email IDs in the thread. An id
/// the server does not find is an entry of `errors`, coded
/// `thread-not-found`, and exits 3.
///
/// JSON output: `{"threads": [<JMAP thread>...], "state", "errors"}`, each
/// thread and the get's `state` as the server sent them.
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
        let output = raw::get(client, "Thread", Some(&self.ids))?;

        let threads = JmapThreadGetOutput {
            account: account.name().to_string(),
            threads: output.list,
            rest: output.rest,
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
    /// The threads, each as the server sent it, in its order.
    #[schemars(with = "Vec<Map<String, Value>>")]
    pub threads: Vec<Value>,
    /// The rest of the server's answer: the get's `state`.
    #[serde(flatten)]
    pub rest: Map<String, Value>,
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
            let id = sanitize::text(thread.get("id").and_then(Value::as_str).unwrap_or_default());
            let email_ids = match thread.get("emailIds") {
                Some(Value::Array(ids)) => ids.iter().filter_map(Value::as_str).collect(),
                _ => Vec::new(),
            };

            if email_ids.is_empty() {
                listing.items.push([id.to_string(), String::new()]);
            }

            for email_id in email_ids {
                listing
                    .items
                    .push([id.to_string(), sanitize::text(email_id).into_owned()]);
            }
        }

        for error in &self.errors {
            listing.errors.push(error);
        }

        write!(f, "{listing}")
    }
}
