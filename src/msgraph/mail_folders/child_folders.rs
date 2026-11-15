//! # Microsoft Graph child folder list
//!
//! The `msgraph mail-folders child-folders` command, listing the folders
//! under one folder.

use anyhow::Result;
use clap::Parser;
use io_msgraph::v1::rest::users::mail_folders::list::MsgraphMailFoldersListParams;
use pimalaya_cli::{clap::args::PageTokenArg, printer::Printer};

use crate::{
    account::context::Account,
    msgraph::{
        client::MsgraphClient,
        mail_folders::list::MsgraphMailFolderListOutput,
        messages::list::{next_page, resume, skip_of},
    },
};

/// List a mail folder's child folders (`GET
/// /me/mailFolders/{id}/childFolders`).
///
/// `nextPage` is Graph's next link as Graph gives it: `--page-token`
/// follows it as is, the query it carries replacing the other options.
#[derive(Debug, Parser)]
pub struct MsgraphChildFoldersListCommand {
    /// The id or well-known name of the parent folder.
    #[arg(value_name = "ID")]
    pub id: String,
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
            None => client.mail_child_folders_list(&self.id, &params)?.response,
        };
        let next = next_page(response.next_link, token.map_or(self.skip, skip_of));

        printer.out(MsgraphMailFolderListOutput::page(
            account,
            response.value,
            next,
        ))
    }
}
