use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Cancel (undo) a pending JMAP email submission (EmailSubmission/set).
///
/// Only submissions with `undoStatus: "pending"` can be canceled.
/// The server may reject this if the message has already been sent.
#[derive(Debug, Parser)]
pub struct CancelSubmissionCommand {
    /// Submission ID(s) to cancel.
    #[arg(value_name = "SUBMISSION_ID", num_args = 1..)]
    pub ids: Vec<String>,
}

impl CancelSubmissionCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        // EmailSubmission/set update: set undoStatus to "canceled"
        let update: std::collections::HashMap<String, serde_json::Value> = self
            .ids
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    serde_json::json!({ "undoStatus": "canceled" }),
                )
            })
            .collect();

        let args = serde_json::json!({
            "update": update
        });

        // Use the raw query approach via SubmitJmapEmail isn't suitable here;
        // we need a direct EmailSubmission/set update. Use the query command
        // pattern with a raw request instead.
        //
        // For now, build the request directly via the send coroutine.
        use io_jmap::{
            coroutines::send::{JmapBatch, SendJmapRequest, SendJmapRequestResult},
            types::session::capabilities,
        };

        let account_id = jmap.context.account_id.clone().unwrap_or_default();
        let api_url = jmap
            .context
            .api_url()
            .cloned()
            .unwrap_or_else(|| "http://localhost".parse().unwrap());

        let mut json_args = args.clone();
        json_args["accountId"] = serde_json::json!(account_id);

        let mut batch = JmapBatch::new();
        batch.add("EmailSubmission/set", json_args);
        let request = batch.into_request(vec![
            capabilities::CORE.into(),
            capabilities::MAIL.into(),
            capabilities::SUBMISSION.into(),
        ]);

        let mut send = SendJmapRequest::new(jmap.context, &api_url, request)
            .map_err(|e| anyhow!("{e}"))?;
        let mut arg = None;

        loop {
            match send.resume(arg.take()) {
                SendJmapRequestResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SendJmapRequestResult::Ok { context, response, .. } => {
                    jmap.context = context;
                    if let Some((name, args, _)) =
                        response.method_responses.into_iter().next()
                    {
                        if name == "error" {
                            bail!("EmailSubmission/set error: {args}");
                        }
                    }
                    break;
                }
                SendJmapRequestResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new(format!(
            "{} submission(s) canceled.",
            self.ids.len()
        )))
    }
}
