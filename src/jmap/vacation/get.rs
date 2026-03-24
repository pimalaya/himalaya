use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::coroutines::vacation_response_get::{
    GetJmapVacationResponse, GetJmapVacationResponseResult,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::account::JmapAccount;

/// Get the JMAP vacation response (VacationResponse/get).
#[derive(Debug, Parser)]
pub struct GetVacationCommand;

impl GetVacationCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut coroutine = GetJmapVacationResponse::new(jmap.context)?;
        let mut arg = None;

        let vacation = loop {
            match coroutine.resume(arg.take()) {
                GetJmapVacationResponseResult::Io(io) => {
                    arg = Some(handle(&mut jmap.stream, io)?)
                }
                GetJmapVacationResponseResult::Ok { context, vacation_response, .. } => {
                    jmap.context = context;
                    break vacation_response;
                }
                GetJmapVacationResponseResult::Err { err, .. } => bail!(err),
            }
        };

        match vacation {
            Some(v) => printer.out(serde_json::to_value(&v)?),
            None => printer.log("No vacation response configured."),
        }
    }
}
