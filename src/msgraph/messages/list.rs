//! # Microsoft Graph message list
//!
//! The `msgraph messages list` command, listing one page of a mailbox or
//! of one folder.

use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_msgraph::v1::rest::users::messages::{
    MsgraphFlagStatus, MsgraphMessage, MsgraphRecipient, list::MsgraphMessagesListParams,
};
use pimalaya_cli::{
    clap::args::PageTokenArg,
    fields::Fields,
    output::Paginated,
    printer::Printer,
    records::{Record, Records},
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, msgraph::client::MsgraphClient};

/// List Microsoft Graph messages (`GET /me/messages` or, with
/// `--folder`, `GET /me/mailFolders/{id}/messages`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageListCommand {
    /// Restrict the listing to a folder id or well-known name (e.g.
    /// `inbox`). Lists the whole mailbox when omitted.
    #[arg(short = 'f', long, value_name = "ID")]
    pub folder: Option<String>,
    /// Maximum number of messages to return (OData `$top`).
    #[arg(long, value_name = "N")]
    pub top: Option<u32>,
    /// Number of messages to skip (OData `$skip`).
    #[arg(long, value_name = "N")]
    pub skip: Option<u32>,
    #[command(flatten)]
    pub page_token: PageTokenArg,
    /// OData `$filter` expression (e.g. `isRead eq false`).
    #[arg(long, value_name = "EXPR")]
    pub filter: Option<String>,
    /// OData `$search` query, `subject:report` or a bare term.
    ///
    /// Graph forbids `$search` beside `$orderby` and ignores `$count`, so
    /// both are dropped here and the results come back in relevance
    /// order.
    #[arg(long, value_name = "QUERY")]
    pub search: Option<String>,
    /// OData `$orderby` expression. Defaults to `receivedDateTime desc`.
    #[arg(long, value_name = "EXPR")]
    pub orderby: Option<String>,
    /// OData `$select`: comma-separated fields to return (e.g.
    /// `subject,from,receivedDateTime`).
    #[arg(long, value_name = "FIELDS")]
    pub select: Option<String>,
    /// Request the total count of matching messages (OData `$count`).
    #[arg(long)]
    pub count: bool,
}

impl MsgraphMessageListCommand {
    /// Lists one page of messages and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let search = self.search.as_deref();

        // NOTE: Graph rejects `$search` beside `$orderby`, and pairing it
        // with `$count` wants a `ConsistencyLevel` header the client does
        // not send, so both are dropped when searching.
        let orderby = match search {
            Some(_) => None,
            None => Some(self.orderby.as_deref().unwrap_or("receivedDateTime desc")),
        };
        let count = match search {
            Some(_) => None,
            None => self.count.then_some(true),
        };

        // NOTE: the token is the `$skip` of the next link Graph returned, so
        // a listing resumes with the same parameters.
        let skip = match &self.page_token.token {
            Some(token) => Some(token.parse::<u32>().map_err(|_| {
                anyhow!("Cannot resume a Microsoft Graph listing from page token {token}")
            })?),
            None => self.skip,
        };

        let params = MsgraphMessagesListParams {
            top: self.top,
            skip,
            select: self.select.as_deref(),
            filter: self.filter.as_deref(),
            search,
            orderby,
            count,
            ..Default::default()
        };
        let response = client
            .messages_list(self.folder.as_deref(), &params)?
            .response;

        // NOTE: a next link without `$skip` (a `$search` page, resumed by
        // `$skiptoken`) comes back whole, which `--page-token` refuses.
        let next_page = response
            .next_link
            .map(|link| skip_of(&link).map_or(link, |skip| skip.to_string()));
        let messages = MsgraphMessageListOutput {
            messages: response.value,
        };

        printer.out(Paginated {
            inner: messages,
            next_page,
        })
    }
}

/// Renders the display form of a recipient: `Name <addr>`, or just the
/// address when there is no display name.
pub(crate) fn recipient(recipient: &MsgraphRecipient) -> String {
    let address = recipient.email_address.address.clone().unwrap_or_default();
    match &recipient.email_address.name {
        Some(name) if !name.is_empty() => format!("{name} <{address}>"),
        _ => address,
    }
}

/// The `msgraph messages list` output, the messages of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MsgraphMessageListOutput {
    messages: Vec<MsgraphMessage>,
}

impl fmt::Display for MsgraphMessageListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut records = Records::default();

        for msg in &self.messages {
            let mut state = Vec::new();
            if msg.is_read == Some(false) {
                state.push(String::from("unread"));
            }
            if msg.flag.as_ref().and_then(|flag| flag.flag_status)
                == Some(MsgraphFlagStatus::Flagged)
            {
                state.push(String::from("flagged"));
            }
            if msg.has_attachments == Some(true) {
                state.push(String::from("attachment"));
            }

            let from = msg.from.as_ref().map(recipient).unwrap_or_default();
            let received = msg.received_date_time.as_deref().unwrap_or_default();
            let subject = msg.subject.as_deref().unwrap_or_default();

            let mut fields = Fields::default();
            fields.push("From", sanitize::text(&from));
            fields.push("Received", sanitize::text(received));

            records.push(Record {
                title: sanitize::text(&msg.id).into_owned(),
                state,
                fields,
                body: sanitize::text(subject).into_owned(),
            });
        }

        write!(f, "{records}")
    }
}

/// The `$skip` query parameter of a Graph next link, percent-encoded or
/// not.
fn skip_of(link: &str) -> Option<u32> {
    let (_, query) = link.split_once('?')?;

    query.split('&').find_map(|param| {
        let (key, value) = param.split_once('=')?;
        matches!(key, "$skip" | "%24skip").then(|| value.parse().ok())?
    })
}

#[cfg(test)]
mod tests {
    use crate::msgraph::messages::list::skip_of;

    #[test]
    fn the_skip_of_a_next_link_is_its_page_token() {
        let link = "https://graph.microsoft.com/v1.0/me/messages?%24top=10&%24skip=20";
        assert_eq!(skip_of(link), Some(20));
        assert_eq!(
            skip_of("https://graph.microsoft.com/v1.0/me/messages?$skip=5"),
            Some(5)
        );
        assert_eq!(
            skip_of("https://graph.microsoft.com/v1.0/me/messages?$skiptoken=x"),
            None
        );
    }
}
