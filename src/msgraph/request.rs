//! # Microsoft Graph request
//!
//! The `msgraph request` command, sending an arbitrary Microsoft Graph
//! request and printing the JSON response: the escape hatch for the
//! endpoints no command covers.

use anyhow::{Context, Result, bail};
use clap::Parser;
use io_msgraph::v1::send::{MSGRAPH_API_BASE as BASE, MsgraphSend};
use pimalaya_cli::printer::Printer;
use serde_json::Value;

use crate::{
    msgraph::client::MsgraphClient,
    shared::raw_json::{RawJsonOutput, api_url},
};

/// Send a raw Microsoft Graph request and print the JSON response.
///
/// The path is joined onto the Graph `v1.0/` base, or a full URL on the
/// same API taken as is, any other host refused before sending. JSON
/// output: `{response}`, the raw Graph response.
#[derive(Debug, Parser)]
pub struct MsgraphRequestCommand {
    /// HTTP method: get, post, patch, delete.
    #[arg(value_name = "METHOD", ignore_case = true)]
    #[arg(value_parser = ["get", "post", "patch", "delete"])]
    pub method: String,
    /// Graph path (joined onto `v1.0/`) or a full URL.
    #[arg(value_name = "PATH", value_parser = path_parser)]
    pub path: String,
    /// Raw JSON body for post and patch.
    #[arg(value_name = "JSON")]
    pub body: Option<String>,
}

impl MsgraphRequestCommand {
    /// Sends the request and prints the raw response.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let url = api_url(BASE, &self.path)?;
        let auth = client.auth.clone();
        let body = || parse_body(self.body.as_deref());

        let send: MsgraphSend<Value> = match self.method.to_ascii_uppercase().as_str() {
            "GET" => MsgraphSend::get(&auth, url),
            "DELETE" => MsgraphSend::delete(&auth, url),
            "POST" => MsgraphSend::post_json(&auth, url, &body()?)?,
            "PATCH" => MsgraphSend::patch_json(&auth, url, &body()?)?,
            other => bail!("Unsupported method `{other}` (use get/post/patch/delete)"),
        };

        printer.out(RawJsonOutput {
            response: client.run(send)?.response,
        })
    }
}

/// Checks a path as clap takes it: a URL off the Graph API is refused
/// with the command line, before the session opens.
fn path_parser(path: &str) -> Result<String, String> {
    match api_url(BASE, path) {
        Ok(_) => Ok(path.to_owned()),
        Err(err) => Err(err.to_string()),
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

    use crate::msgraph::{client::MsgraphClient, request::MsgraphRequestCommand};

    /// The path joins the `v1.0/` base, the body goes as sent and the
    /// response prints raw under `response`.
    #[test]
    fn a_request_prints_the_raw_response() {
        let answer = json!({"id": "F1", "x-vendor": 1});
        let (mut client, requests) = MsgraphClient::scripted(std::slice::from_ref(&answer));
        let mut printer = CapturePrinter::default();

        let command = MsgraphRequestCommand {
            method: "patch".into(),
            path: "/me/mailFolders/F1".into(),
            body: Some(r#"{"displayName":"Work"}"#.into()),
        };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), json!({"response": answer}));
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("PATCH /v1.0/me/mailFolders/F1 "));
        assert!(requests[0].ends_with(r#"{"displayName":"Work"}"#));
    }
}
