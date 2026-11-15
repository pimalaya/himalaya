//! # Microsoft Graph mail folder list
//!
//! The `msgraph mail-folders list` command, listing the folders of the
//! mailbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::{
    query::to_query_pairs,
    rest::users::mail_folders::{MsgraphMailFolder, list::MsgraphMailFoldersListParams},
};
use pimalaya_cli::{
    clap::args::PageTokenArg,
    listing::{Listing, ListingPage},
    printer::Printer,
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    backend::Backend,
    listing,
    msgraph::{
        client::MsgraphClient,
        messages::list::{next_page, raw_page, resume, skip_of},
    },
};

/// List Microsoft Graph mail folders (`GET /me/mailFolders`).
///
/// `nextPage` is Graph's next link as Graph gives it: `--page-token`
/// follows it as is, carrying the query, so no other option goes beside
/// it.
///
/// JSON output: `{"folders": [<raw Graph mail folder>...], "nextPage"}`.
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderListCommand {
    /// Maximum number of folders to return (OData `$top`).
    #[arg(long, value_name = "N", conflicts_with = "page_token")]
    pub top: Option<u32>,
    /// Number of folders to skip (OData `$skip`).
    #[arg(long, value_name = "N", conflicts_with = "page_token")]
    pub skip: Option<u32>,
    #[command(flatten)]
    pub page_token: PageTokenArg,
    /// OData `$select`: comma-separated fields to return (e.g.
    /// `displayName,totalItemCount`).
    #[arg(long, value_name = "FIELDS", conflicts_with = "page_token")]
    pub select: Option<String>,
    /// Also include hidden folders.
    #[arg(long, conflicts_with = "page_token")]
    pub hidden: bool,
}

impl MsgraphMailFolderListCommand {
    /// Lists one page of folders and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let params = MsgraphMailFoldersListParams {
            top: self.top,
            skip: self.skip,
            select: self.select.as_deref(),
            include_hidden_folders: self.hidden.then_some(true),
        };

        let token = self.page_token.token.as_deref();
        let response = match token {
            Some(link) => resume(client, link)?,
            None => raw_page(client, "mailFolders", &to_query_pairs(&params))?,
        };
        let next = next_page(response.next_link, token.map_or(self.skip, skip_of));

        printer.out(MsgraphMailFolderListOutput::page(
            account,
            response.value,
            next,
        ))
    }
}

/// The Microsoft Graph mail folders, as `list`, `get` and
/// `child-folders` print them.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MsgraphMailFolderListOutput {
    #[serde(skip)]
    account: String,
    /// The folders, each as Graph sent it, every property it carries
    /// kept.
    #[schemars(with = "Vec<Map<String, Value>>")]
    folders: Vec<Value>,
    /// Graph's next link, `null` once the listing is complete and for
    /// `get`, given back through `--page-token`.
    next_page: Option<String>,
    #[serde(skip)]
    paged: bool,
}

impl MsgraphMailFolderListOutput {
    /// Wraps the folders the server returned for `account`.
    pub fn new(account: &Account, folders: Vec<MsgraphMailFolder>) -> Self {
        let folders = folders.iter().filter_map(|f| serde_json::to_value(f).ok());
        Self {
            account: account.name().to_string(),
            folders: folders.collect(),
            next_page: None,
            paged: false,
        }
    }

    /// Wraps one page of the folders the server listed for `account`,
    /// `next` resuming the listing.
    pub fn page(account: &Account, folders: Vec<Value>, next: Option<String>) -> Self {
        Self {
            account: account.name().to_string(),
            folders,
            next_page: next,
            paged: true,
        }
    }
}

impl fmt::Display for MsgraphMailFolderListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Msgraph.as_str()));

        for folder in &self.folders {
            let folder: MsgraphMailFolder =
                serde_json::from_value(folder.clone()).unwrap_or_default();
            listing.items.push([
                sanitize::text(&folder.id).into_owned(),
                listing::counts(folder.unread_item_count, folder.total_item_count),
                count(folder.child_folder_count),
                sanitize::text(&folder.display_name).into_owned(),
            ]);
        }

        if self.paged {
            let count = self.folders.len();
            let next = self.next_page.as_deref();
            listing.page(ListingPage::Token, count, ["folder", "folders"], next);
        }

        write!(f, "{listing}")
    }
}

/// Renders a count, or nothing when Graph gave none.
fn count(value: Option<impl ToString>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}
