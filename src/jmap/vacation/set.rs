use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::{
    coroutines::vacation_response_set::{
        SetJmapVacationResponse, SetJmapVacationResponseResult,
    },
    types::vacation_response::VacationResponseUpdate,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Update the JMAP vacation response (VacationResponse/set).
#[derive(Debug, Parser)]
pub struct SetVacationCommand {
    /// Enable the vacation response.
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,

    /// Disable the vacation response.
    #[arg(long, conflicts_with = "enable")]
    pub disable: bool,

    /// Active from date (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub from_date: Option<String>,

    /// Active until date (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub to_date: Option<String>,

    /// Subject line for the auto-reply.
    #[arg(long, value_name = "TEXT")]
    pub subject: Option<String>,

    /// Plaintext body for the auto-reply.
    #[arg(long, value_name = "TEXT")]
    pub text_body: Option<String>,

    /// HTML body for the auto-reply.
    #[arg(long, value_name = "TEXT")]
    pub html_body: Option<String>,
}

impl SetVacationCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let is_enabled = if self.enable {
            Some(true)
        } else if self.disable {
            Some(false)
        } else {
            None
        };

        let patch = VacationResponseUpdate {
            is_enabled,
            from_date: self.from_date,
            to_date: self.to_date,
            subject: self.subject,
            text_body: self.text_body,
            html_body: self.html_body,
        };

        let mut coroutine = SetJmapVacationResponse::new(jmap.context, patch)?;
        let mut arg = None;

        loop {
            match coroutine.resume(arg.take()) {
                SetJmapVacationResponseResult::Io(io) => {
                    arg = Some(handle(&mut jmap.stream, io)?)
                }
                SetJmapVacationResponseResult::Ok { context, .. } => {
                    jmap.context = context;
                    break;
                }
                SetJmapVacationResponseResult::Err { err, .. } => bail!(err),
            }
        }

        printer.out(Message::new("Vacation response updated."))
    }
}
