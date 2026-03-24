use std::{
    fmt,
    io::{stdin, BufRead},
};

use anyhow::{bail, Context, Result};
use clap::Parser;
use io_jmap::coroutines::send::{JmapRequest, SendJmapRequest, SendJmapRequestResult};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::jmap::account::JmapAccount;

/// Send a raw JMAP method-calls array and print the response.
///
/// METHOD_CALLS must be a JSON array of JMAP method call tuples:
///
///   '[["Mailbox/query", {"filter": {"role": "inbox"}}, "c0"]]'
///
/// The `accountId` field is injected into each call's arguments
/// automatically if not already present. Pass `-` or omit to read
/// from stdin.
#[derive(Debug, Parser)]
pub struct QueryCommand {
    /// Extra capability URNs to declare (core and mail are always included).
    #[arg(long = "using", value_name = "URN")]
    pub using: Vec<String>,

    /// The JMAP methodCalls JSON array (or omit / pass `-` to read stdin).
    #[arg(trailing_var_arg = true)]
    #[arg(name = "method-calls", value_name = "METHOD_CALLS")]
    pub method_calls: Vec<String>,
}

impl QueryCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let raw = if self.method_calls.is_empty()
            || self.method_calls.first().map(|s| s.as_str()) == Some("-")
        {
            stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            self.method_calls.join(" ")
        };

        let calls_value: serde_json::Value =
            serde_json::from_str(&raw).context("METHOD_CALLS is not valid JSON")?;

        let serde_json::Value::Array(calls_arr) = calls_value else {
            bail!("METHOD_CALLS must be a JSON array");
        };

        let account_id = jmap.context.account_id.clone().unwrap_or_default();

        // Parse and inject accountId into each call's args.
        let mut method_calls = Vec::with_capacity(calls_arr.len());
        for (i, call) in calls_arr.into_iter().enumerate() {
            let serde_json::Value::Array(mut tuple) = call else {
                bail!("method call #{i} must be a JSON array [name, args, callId]");
            };
            if tuple.len() != 3 {
                bail!("method call #{i} must have exactly 3 elements [name, args, callId]");
            }
            let call_id = match tuple.remove(2) {
                serde_json::Value::String(s) => s,
                v => bail!("method call #{i} callId must be a string, got {v}"),
            };
            let mut args = tuple.remove(1);
            let name = match tuple.remove(0) {
                serde_json::Value::String(s) => s,
                v => bail!("method call #{i} name must be a string, got {v}"),
            };
            // Inject accountId if the args object doesn't already have it.
            if let serde_json::Value::Object(ref mut map) = args {
                map.entry("accountId")
                    .or_insert_with(|| serde_json::Value::String(account_id.clone()));
            }
            method_calls.push((name, args, call_id));
        }

        let mut using = vec![
            io_jmap::types::session::capabilities::CORE.to_string(),
            io_jmap::types::session::capabilities::MAIL.to_string(),
        ];
        for extra in self.using {
            if !using.contains(&extra) {
                using.push(extra);
            }
        }

        let api_url = jmap
            .context
            .api_url()
            .cloned()
            .unwrap_or_else(|| "http://localhost".parse().unwrap());

        let request = JmapRequest {
            using,
            method_calls,
            created_ids: None,
        };

        let mut coroutine = SendJmapRequest::new(jmap.context, &api_url, request)?;
        let mut arg = None;

        let response = loop {
            match coroutine.resume(arg.take()) {
                SendJmapRequestResult::Ok { context, response, .. } => {
                    jmap.context = context;
                    break response;
                }
                SendJmapRequestResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SendJmapRequestResult::Err { err, .. } => return Err(err.into()),
            }
        };

        printer.out(RawResponse(response.method_responses))
    }
}

/// Wraps the raw method_responses for display.
#[derive(Serialize)]
struct RawResponse(Vec<(String, serde_json::Value, String)>);

impl fmt::Display for RawResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string_pretty(&self.0) {
            Ok(s) => write!(f, "{s}"),
            Err(e) => write!(f, "<serialization error: {e}>"),
        }
    }
}
