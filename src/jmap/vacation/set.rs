use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::{
    capabilities::VACATION_RESPONSE,
    vacation_response::VacationResponseUpdate,
    vacation_response_set::{JmapVacationResponseSet, JmapVacationResponseSetResult},
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Update the JMAP vacation response (VacationResponse/set).
#[derive(Debug, Parser)]
pub struct JmapVacationSetCommand {
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

impl JmapVacationSetCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        // Skip the request if the server does not advertise the
        // vacation-response capability.
        let has_vacation = jmap.session.capabilities.contains_key(VACATION_RESPONSE);

        if !has_vacation {
            bail!("Vacation response is not supported by the server");
        }

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

        let mut coroutine = JmapVacationResponseSet::new(&jmap.session, &jmap.http_auth, patch)?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                JmapVacationResponseSetResult::Ok { .. } => break,
                JmapVacationResponseSetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapVacationResponseSetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapVacationResponseSetResult::Err(err) => bail!("{err}"),
            }
        }

        printer.out(Message::new("Vacation response successfully updated"))
    }
}
