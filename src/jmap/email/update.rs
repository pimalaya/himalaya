use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_set::{EmailSetArgs, SetJmapEmails, SetJmapEmailsResult};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Update JMAP emails via patch operations (Email/set).
#[derive(Debug, Parser)]
pub struct JmapEmailUpdateCommand {
    /// Email ID(s) to update.
    #[arg(value_name = "EMAIL_ID", required = true, num_args = 1..)]
    pub ids: Vec<String>,

    /// Add keyword(s) to the email(s).
    #[arg(long, value_name = "KEYWORD", num_args = 0..)]
    pub add_keyword: Vec<String>,

    /// Remove keyword(s) from the email(s).
    #[arg(long, value_name = "KEYWORD", num_args = 0..)]
    pub remove_keyword: Vec<String>,

    /// Replace all keywords atomically (no fetch required).
    #[arg(long, value_name = "KEYWORD", num_args = 0..)]
    pub keywords: Option<Vec<String>>,

    /// Add email(s) to a mailbox.
    #[arg(long, value_name = "MAILBOX-ID", num_args = 1..)]
    pub add_mailbox: Vec<String>,

    /// Remove email(s) from a mailbox.
    #[arg(long, value_name = "MAILBOX-ID", num_args = 1..)]
    pub remove_mailbox: Vec<String>,

    /// Replace all mailbox memberships atomically.
    #[arg(long, value_name = "MAILBOX-ID", num_args = 0..)]
    pub mailboxes: Option<Vec<String>>,
}

impl JmapEmailUpdateCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;
        let mut args = EmailSetArgs::default();

        for id in &self.ids {
            for kw in &self.add_keyword {
                args.set_keyword(id.clone(), kw.clone());
            }

            for kw in &self.remove_keyword {
                args.unset_keyword(id.clone(), kw.clone());
            }

            if let Some(kws) = &self.keywords {
                let map: HashMap<String, bool> = kws.iter().map(|kw| (kw.clone(), true)).collect();
                args.replace_keywords(id.clone(), map);
            }

            for mbox in &self.add_mailbox {
                args.add_to_mailbox(id.clone(), mbox.clone());
            }

            for mbox in &self.remove_mailbox {
                args.remove_from_mailbox(id.clone(), mbox.clone());
            }

            if let Some(mboxes) = &self.mailboxes {
                let map: HashMap<String, bool> = mboxes.iter().map(|m| (m.clone(), true)).collect();
                args.replace_mailbox_ids(id.clone(), map);
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
            let mut ctx = anyhow!("Failed to update email `{id}`");

            if let Some(desc) = &err.description {
                ctx = anyhow!(desc.clone()).context(ctx);
            }

            if !err.properties.is_empty() {
                ctx = anyhow!("Invalid properties: {}", err.properties.join(", ")).context(ctx);
            }

            bail!(ctx);
        }

        printer.out(Message::new("Email(s) successfully updated"))
    }
}
