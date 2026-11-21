//! # JMAP request
//!
//! The `jmap request` command, sending a caller-supplied JMAP request
//! object and printing the raw response.

use std::{
    fs,
    io::{Read, stdin},
    path::Path,
};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use io_jmap::{
    rfc8620::{JMAP_CORE_CAPABILITY, request::JmapRequest},
    rfc8621::JMAP_MAIL_CAPABILITY,
};
use pimalaya_cli::{printer::Printer, session::check_stdin};
use serde_json::{Map, Value, json};

use crate::{error::ErrorCode, jmap::client::JmapClient, raw_json::RawJsonOutput};

/// Send a raw JMAP request and print the raw JMAP response.
///
/// The body is a JMAP request object (`{"using": [...], "methodCalls":
/// [...]}`), so any method can be driven with custom `using`
/// capabilities. `using` defaults to the core and mail capabilities when
/// omitted.
///
/// JSON output: `{response}`, the raw JMAP response.
#[derive(Debug, Parser)]
pub struct JmapRequestCommand {
    /// A path to a JSON file, raw JSON, or `-` for stdin.
    #[arg(value_name = "JSON")]
    pub body: String,
}

impl JmapRequestCommand {
    /// Sends the request and prints the raw response.
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let map = read_object(self.body)?;

        let using = match map.get("using") {
            Some(value) => {
                serde_json::from_value(value.clone()).context("Invalid `using` array")?
            }
            None => vec![
                JMAP_CORE_CAPABILITY.to_string(),
                JMAP_MAIL_CAPABILITY.to_string(),
            ],
        };
        let method_calls = map
            .get("methodCalls")
            .cloned()
            .ok_or_else(|| anyhow!("JMAP request needs a `methodCalls` array"))?;
        let method_calls =
            serde_json::from_value(method_calls).context("Invalid `methodCalls` array")?;

        let request = JmapRequest {
            using,
            method_calls,
            created_ids: None,
        };
        let response = client.send_raw(request)?;

        let response = json!({
            "methodResponses": response.method_responses,
            "createdIds": response.created_ids,
            "sessionState": response.session_state,
        });

        printer.out(RawJsonOutput { response })
    }
}

/// Reads `body` into a JSON object: `-` reads stdin, an existing file is
/// read, otherwise the value is the JSON itself.
fn read_object(body: String) -> Result<Map<String, Value>> {
    let raw = if body == "-" {
        check_stdin(ErrorCode::StdinTaken)?;
        let mut buf = String::new();
        stdin()
            .read_to_string(&mut buf)
            .context("Read JSON from stdin error")?;
        buf
    } else if Path::new(&body).is_file() {
        fs::read_to_string(&body).with_context(|| format!("Read JSON from `{body}` error"))?
    } else {
        body
    };

    match serde_json::from_str(&raw).context("Parse JSON error")? {
        Value::Object(map) => Ok(map),
        _ => bail!("The JSON body must be an object"),
    }
}
