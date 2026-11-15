//! # Gmail request
//!
//! The `gmail request` command, sending an arbitrary Gmail API request
//! and printing the JSON response: the escape hatch for the endpoints no
//! command covers.

use anyhow::{Context, Result, bail};
use clap::Parser;
use io_gmail::v1::send::{GMAIL_API_BASE as BASE, GmailSend};
use pimalaya_cli::printer::Printer;
use serde_json::Value;

use crate::{
    gmail::client::GmailClient,
    shared::raw_json::{RawJsonOutput, api_url},
};

/// Send a raw Gmail API request and print the JSON response.
///
/// The path is joined onto the Gmail `gmail/v1/` base, or a full URL on the
/// same API taken as is, any other host refused before sending. JSON
/// output: `{response}`, the raw Gmail response.
#[derive(Debug, Parser)]
pub struct GmailRequestCommand {
    /// HTTP method: get, post, put, patch, delete.
    #[arg(value_name = "METHOD")]
    pub method: String,
    /// Gmail path (joined onto `gmail/v1/`, as `users/me/labels`) or a
    /// full URL.
    #[arg(value_name = "PATH")]
    pub path: String,
    /// Raw JSON body for post, put and patch.
    #[arg(value_name = "JSON")]
    pub body: Option<String>,
}

impl GmailRequestCommand {
    /// Sends the request and prints the raw response.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let url = api_url(BASE, &self.path)?;
        let auth = client.auth.clone();
        let body = || parse_body(self.body.as_deref());

        let send: GmailSend<Value> = match self.method.to_ascii_uppercase().as_str() {
            "GET" => GmailSend::get(&auth, url)?,
            "DELETE" => GmailSend::delete(&auth, url)?,
            "POST" => GmailSend::post_json(&auth, url, &body()?)?,
            "PUT" => GmailSend::put_json(&auth, url, &body()?)?,
            "PATCH" => GmailSend::patch_json(&auth, url, &body()?)?,
            other => bail!("Unsupported method `{other}` (use get/post/put/patch/delete)"),
        };

        printer.out(RawJsonOutput {
            response: client.run(send)?.response,
        })
    }
}

/// Parses the raw body argument, an absent body meaning JSON `null`.
fn parse_body(body: Option<&str>) -> Result<Value> {
    serde_json::from_str(body.unwrap_or("null")).context("Parse request body JSON error")
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::gmail::{client::GmailClient, request::GmailRequestCommand};

    /// The path joins the `gmail/v1/` base and the response prints raw
    /// under `response`.
    #[test]
    fn a_request_prints_the_raw_response() {
        let answer = json!({"labels": [{"id": "L1", "x-vendor": 1}]});
        let (mut client, requests) = GmailClient::scripted(std::slice::from_ref(&answer));
        let mut printer = CapturePrinter::default();

        let command = GmailRequestCommand {
            method: "get".into(),
            path: "users/me/labels".into(),
            body: None,
        };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), json!({"response": answer}));
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("GET /gmail/v1/users/me/labels "));
    }
}
