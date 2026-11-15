//! # JMAP submission query
//!
//! The `jmap submission query` command, an RFC 8621
//! `EmailSubmission/query` chained into an `EmailSubmission/get`.

use std::fmt;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use io_jmap::rfc8621::email_submission::{
    JmapEmailSubmission, JmapUndoStatus,
    query::{JmapEmailSubmissionFilter, JmapEmailSubmissionQueryOptions},
};
use pimalaya_cli::{
    clap::args::PageTokenArg,
    listing::{Listing, ListingPage},
    printer::Printer,
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    error::{ErrorCode, ItemError, partial_when},
    jmap::{
        backend::{not_found_errors, query_page_follows},
        client::JmapClient,
    },
};

/// How far along a submission is, and whether it can still be canceled.
#[derive(Clone, Debug, ValueEnum)]
pub enum UndoStatusArg {
    /// Not sent yet, so still cancelable.
    Pending,
    /// Sent, so no longer cancelable.
    Final,
    /// Canceled before it was sent.
    Canceled,
}

impl From<UndoStatusArg> for JmapUndoStatus {
    fn from(arg: UndoStatusArg) -> Self {
        match arg {
            UndoStatusArg::Pending => JmapUndoStatus::Pending,
            UndoStatusArg::Final => JmapUndoStatus::Final,
            UndoStatusArg::Canceled => JmapUndoStatus::Canceled,
        }
    }
}

/// Query JMAP email submissions (EmailSubmission/query + EmailSubmission/get).
#[derive(Debug, Parser)]
pub struct JmapSubmissionQueryCommand {
    /// Filter by undo status (`pending`, `final`, `canceled`).
    #[arg(long, value_name = "STATUS")]
    pub undo_status: Option<UndoStatusArg>,
    /// Filter by sent-before date (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub before: Option<String>,
    /// Filter by sent-after date (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub after: Option<String>,
    /// Number of submissions to display per page.
    #[arg(long, short = 's', value_name = "N", default_value = "10")]
    pub page_size: u64,
    /// Page index, starting from 1.
    #[arg(long, short, value_name = "N", default_value = "1")]
    pub page: u64,
    #[command(flatten)]
    pub page_token: PageTokenArg,
}

impl JmapSubmissionQueryCommand {
    /// Queries the submissions and prints the page it returned.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let page = match &self.page_token.token {
            Some(token) => token
                .parse()
                .ok()
                .filter(|page: &u64| *page > 0)
                .with_context(|| format!("Invalid page token {token}"))?,
            None => self.page.max(1),
        };

        let filter = {
            let f = JmapEmailSubmissionFilter {
                undo_status: self.undo_status.map(Into::into),
                before: self.before,
                after: self.after,
                ..Default::default()
            };

            let has_one = f.undo_status.is_some() || f.before.is_some() || f.after.is_some();

            if has_one { Some(f) } else { None }
        };

        let output = client.email_submission_query(JmapEmailSubmissionQueryOptions {
            filter,
            sort: None,
            position: Some((page - 1).saturating_mul(self.page_size)),
            limit: Some(self.page_size),
        })?;

        let listed = output.submissions.len() + output.not_found.len();
        let follows = query_page_follows(output.total, listed, page, self.page_size);

        let submissions = JmapSubmissionQueryOutput {
            account: account.name().to_string(),
            page: Some((page, self.page_size)),
            submissions: output.submissions,
            next_page: follows.then(|| (page + 1).to_string()),
            errors: not_found_errors(output.not_found, ErrorCode::SubmissionNotFound),
        };

        printer.out(&submissions)?;
        partial_when(&submissions.errors)
    }
}

/// The `jmap submission query`, `get` and `create` output, the
/// submissions of one page, of the ids given or created.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JmapSubmissionQueryOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The page listed and its size, `None` for a get or a create.
    #[serde(skip)]
    pub page: Option<(u64, u64)>,
    /// The submissions, in the order the server returned them.
    pub submissions: Vec<JmapEmailSubmission>,
    /// The number of the next page, `null` once the query is complete and
    /// for a get or a create, given back through `--page-token`.
    pub next_page: Option<String>,
    /// The error of each submission the server did not find, in the order
    /// met, none for a create.
    pub errors: Vec<ItemError>,
}

impl fmt::Display for JmapSubmissionQueryOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Jmap.as_str()));

        for submission in &self.submissions {
            let text = |value: &Option<String>| {
                sanitize::text(value.as_deref().unwrap_or_default()).into_owned()
            };
            let status = submission.undo_status.as_ref().map(ToString::to_string);

            listing.items.push([
                text(&submission.id),
                text(&submission.email_id),
                text(&submission.identity_id),
                status.unwrap_or_default(),
                text(&submission.send_at),
            ]);
        }

        if let Some((page, size)) = self.page {
            let count = self.submissions.len();
            let page = ListingPage::Numbered { page, size };
            let nouns = ["submission", "submissions"];
            listing.page(page, count, nouns, self.next_page.as_deref());
        }

        for error in &self.errors {
            listing.errors.push(error);
        }

        write!(f, "{listing}")
    }
}
