//! # Microsoft Graph child folder list
//!
//! The `msgraph mail-folders child-folders` command, listing the folders
//! under one folder.

use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::{
    query::to_query_pairs, rest::users::mail_folders::list::MsgraphMailFoldersListParams,
    send::path_segment,
};
use pimalaya_cli::{clap::args::PageTokenArg, printer::Printer};

use crate::{
    account::context::Account,
    msgraph::{
        client::MsgraphClient,
        mail_folders::list::MsgraphMailFolderListOutput,
        messages::list::{next_page, raw_page, resume, skip_of},
    },
};

/// List a mail folder's child folders (`GET
/// /me/mailFolders/{id}/childFolders`).
///
/// `nextPage` is Graph's next link as Graph gives it: `--page-token`
/// follows it as is, carrying the query, so no other option goes beside
/// it.
///
/// JSON output: `{"folders": [<raw Graph mail folder>...], "nextPage"}`.
#[derive(Debug, Parser)]
pub struct MsgraphChildFoldersListCommand {
    /// The id or well-known name of the parent folder, which a page token
    /// carries.
    #[arg(value_name = "ID", required_unless_present = "page_token")]
    #[arg(conflicts_with = "page_token")]
    pub id: Option<String>,
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

impl MsgraphChildFoldersListCommand {
    /// Lists the folders under the given one and prints them.
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
            None => {
                let id = self.id.as_deref().unwrap_or_default();
                let path = format!("mailFolders/{}/childFolders", path_segment(id)?);
                raw_page(client, &path, &to_query_pairs(&params))?
            }
        };
        let next = next_page(response.next_link, token.map_or(self.skip, skip_of));

        printer.out(MsgraphMailFolderListOutput::page(
            account,
            response.value,
            next,
        ))
    }
}
