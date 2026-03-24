use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::coroutines::thread_get::{GetJmapThreads, GetJmapThreadsResult};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::account::JmapAccount;

/// Get JMAP threads by ID (Thread/get).
///
/// Each thread contains an ordered list of email IDs in the thread.
#[derive(Debug, Parser)]
pub struct GetThreadCommand {
    /// Thread ID(s) to retrieve.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl GetThreadCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine = GetJmapThreads::new(jmap.context, self.ids.clone())?;
        let mut arg = None;

        let (threads, not_found) = loop {
            match coroutine.resume(arg.take()) {
                GetJmapThreadsResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                GetJmapThreadsResult::Ok {
                    threads, not_found, ..
                } => break (threads, not_found),
                GetJmapThreadsResult::Err { err, .. } => bail!(err),
            }
        };

        for id in &not_found {
            printer.log(format!("Thread `{id}` not found."))?;
        }

        for thread in threads {
            printer.out(serde_json::json!({
                "id": thread.id,
                "emailIds": thread.email_ids,
            }))?;
        }

        Ok(())
    }
}
