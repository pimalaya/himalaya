use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_set::{EmailSetArgs, SetJmapEmails, SetJmapEmailsResult};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::account::JmapAccount;

/// Add keywords to JMAP emails.
///
/// Standard JMAP keywords: `$seen`, `$flagged`, `$answered`, `$draft`.
#[derive(Debug, Parser)]
pub struct AddKeywordCommand {
    /// Email ID(s) to add the keyword(s) to.
    #[arg(value_name = "EMAIL_ID", num_args = 1..)]
    pub ids: Vec<String>,

    /// The keyword(s) to add.
    #[arg(long, short, num_args = 1..)]
    pub keyword: Vec<String>,
}

impl AddKeywordCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut args = EmailSetArgs::default();

        for id in &self.ids {
            for kw in &self.keyword {
                args.set_keyword(id.clone(), kw.clone());
            }
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
            let mut ctx = anyhow!("failed to add keyword to email `{id}`");
            if let Some(desc) = &err.description {
                ctx = anyhow!(desc.clone()).context(ctx);
            }
            if !err.properties.is_empty() {
                ctx = anyhow!("invalid properties: {}", err.properties.join(", ")).context(ctx);
            }
            bail!(ctx);
        }

        printer.log(format!(
            "Keyword(s) `{}` added to {} email(s).",
            self.keyword.join(", "),
            self.ids.len()
        ))
    }
}
