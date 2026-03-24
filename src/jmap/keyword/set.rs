use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_set::{EmailSetArgs, SetJmapEmails, SetJmapEmailsResult};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::account::JmapAccount;

/// Replace all keywords on JMAP emails.
///
/// Replaces the entire set of keywords atomically — no need to know
/// the current keywords first.
#[derive(Debug, Parser)]
pub struct SetKeywordsCommand {
    /// Email ID(s) to update.
    #[arg(value_name = "EMAIL_ID", num_args = 1..)]
    pub ids: Vec<String>,

    /// Keywords to set (replaces all existing keywords).
    #[arg(long, short, num_args = 1..)]
    pub keyword: Vec<String>,
}

impl SetKeywordsCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let keywords: HashMap<String, bool> =
            self.keyword.iter().map(|kw| (kw.clone(), true)).collect();

        let mut args = EmailSetArgs::default();
        for id in &self.ids {
            args.replace_keywords(id.clone(), keywords.clone());
        }

        let mut coroutine = SetJmapEmails::new(jmap.context, args)?;
        let mut arg = None;

        let not_updated = loop {
            match coroutine.resume(arg.take()) {
                SetJmapEmailsResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SetJmapEmailsResult::Ok { not_updated, .. } => break not_updated,
                SetJmapEmailsResult::Err { err, .. } => bail!(err),
            }
        };

        for (id, err) in &not_updated {
            let mut ctx = anyhow!("failed to set keywords on email `{id}`");
            if let Some(desc) = &err.description {
                ctx = anyhow!(desc.clone()).context(ctx);
            }
            if !err.properties.is_empty() {
                ctx = anyhow!("invalid properties: {}", err.properties.join(", ")).context(ctx);
            }
            bail!(ctx);
        }

        printer.log(format!("Keywords set on {} email(s).", self.ids.len()))
    }
}
