use std::collections::BTreeMap;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::email::set::JmapEmailSetArgs;
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{client::JmapClient, error::format_set_error};

/// Update JMAP emails via patch operations (Email/set).
#[derive(Debug, Parser)]
pub struct JmapEmailUpdateCommand {
    /// Email ID(s) to update.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,

    /// Add keyword(s) to the email(s).
    #[arg(long, value_name = "KEYWORD", required = false)]
    pub add_keyword: Vec<String>,

    /// Remove keyword(s) from the email(s).
    #[arg(long, value_name = "KEYWORD", required = false)]
    pub remove_keyword: Vec<String>,

    /// Replace all keywords atomically.
    #[arg(long, value_name = "KEYWORD")]
    pub keywords: Option<Vec<String>>,

    /// Add email(s) to a mailbox.
    #[arg(long, value_name = "MAILBOX-ID", required = false)]
    pub add_mailbox: Vec<String>,

    /// Remove email(s) from a mailbox.
    #[arg(long, value_name = "MAILBOX-ID", required = false)]
    pub remove_mailbox: Vec<String>,

    /// Replace all mailbox memberships atomically.
    #[arg(long, value_name = "MAILBOX-ID")]
    pub mailboxes: Option<Vec<String>>,
}

impl JmapEmailUpdateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let mut args = JmapEmailSetArgs::default();

        for id in &self.ids {
            for kw in &self.add_keyword {
                args.set_keyword(id.clone(), kw.clone());
            }

            for kw in &self.remove_keyword {
                args.unset_keyword(id.clone(), kw.clone());
            }

            if let Some(kws) = &self.keywords {
                let map: BTreeMap<String, bool> = kws.iter().map(|kw| (kw.clone(), true)).collect();
                args.replace_keywords(id.clone(), map);
            }

            for mbox in &self.add_mailbox {
                args.add_to_mailbox(id.clone(), mbox.clone());
            }

            for mbox in &self.remove_mailbox {
                args.remove_from_mailbox(id.clone(), mbox.clone());
            }

            if let Some(mboxes) = &self.mailboxes {
                let map: BTreeMap<String, bool> =
                    mboxes.iter().map(|m| (m.clone(), true)).collect();
                args.replace_mailbox_ids(id.clone(), map);
            }
        }

        let output = client.email_set(args)?;

        if !output.not_updated.is_empty() {
            let mut msg = String::from("Update JMAP email(s) error");

            for (id, err) in output.not_updated {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(&err));
            }

            bail!(msg)
        }

        printer.out(Message::new("Email(s) successfully updated"))
    }
}
