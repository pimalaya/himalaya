//! # Microsoft Graph mail folder list
//!
//! The `msgraph mail-folders list` command, listing the folders of the
//! mailbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::mail_folders::{
    MsgraphMailFolder, list::MsgraphMailFoldersListParams,
};
use pimalaya_cli::{clap::args::PageTokenArg, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    listing::{self, Listing, ListingPage},
    msgraph::{
        client::MsgraphClient,
        messages::list::{next_page_token, page_skip},
    },
};

/// List Microsoft Graph mail folders (`GET /me/mailFolders`).
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderListCommand {
    /// Maximum number of folders to return (OData `$top`).
    #[arg(long, value_name = "N")]
    pub top: Option<u32>,
    /// Number of folders to skip (OData `$skip`).
    #[arg(long, value_name = "N")]
    pub skip: Option<u32>,
    #[command(flatten)]
    pub page_token: PageTokenArg,
    /// OData `$select`: comma-separated fields to return (e.g.
    /// `displayName,totalItemCount`).
    #[arg(long, value_name = "FIELDS")]
    pub select: Option<String>,
    /// Also include hidden folders.
    #[arg(long)]
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
            skip: page_skip(self.page_token.token.as_deref(), self.skip)?,
            select: self.select.as_deref(),
            include_hidden_folders: self.hidden.then_some(true),
        };
        let response = client.mail_folders_list(&params)?.response;
        let next = next_page_token(response.next_link, params.skip);

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
    folders: Vec<MsgraphMailFolder>,
    /// The cursor of the next page, `null` once the listing is complete
    /// and for `get`, given back through `--page-token`.
    next_page: Option<String>,
    #[serde(skip)]
    paged: bool,
}

impl MsgraphMailFolderListOutput {
    /// Wraps the folders the server returned for `account`.
    pub fn new(account: &Account, folders: Vec<MsgraphMailFolder>) -> Self {
        Self {
            account: account.name().to_string(),
            folders,
            next_page: None,
            paged: false,
        }
    }

    /// Wraps one page of the folders the server listed for `account`,
    /// `next` resuming the listing.
    pub fn page(account: &Account, folders: Vec<MsgraphMailFolder>, next: Option<String>) -> Self {
        Self {
            next_page: next,
            paged: true,
            ..Self::new(account, folders)
        }
    }
}

impl fmt::Display for MsgraphMailFolderListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Msgraph));

        for folder in &self.folders {
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
