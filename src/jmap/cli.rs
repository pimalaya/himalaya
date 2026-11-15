//! # JMAP command
//!
//! The `jmap` command, dispatching onto its subcommand groups.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    jmap::{
        client::JmapClient, email::cli::JmapEmailCommand, identity::cli::JmapIdentityCommand,
        mailbox::cli::JmapMailboxCommand, query::JmapQueryCommand, request::JmapRequestCommand,
        submission::cli::JmapSubmissionCommand, thread::cli::JmapThreadCommand,
        vacation::cli::JmapVacationCommand,
    },
};

/// JMAP-specific API.
///
/// Each subcommand carries the name of its JMAP counterpart, grouped by
/// data type.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum JmapCommand {
    Request(JmapRequestCommand),
    /// Manage JMAP mailboxes.
    #[command(subcommand, visible_aliases = ["mbox"])]
    Mailbox(JmapMailboxCommand),
    /// Manage JMAP emails.
    #[command(subcommand)]
    Email(JmapEmailCommand),
    /// Manage JMAP threads.
    #[command(subcommand)]
    Thread(JmapThreadCommand),
    /// Manage JMAP sender identities.
    #[command(subcommand)]
    Identity(JmapIdentityCommand),
    /// Manage JMAP email submissions.
    #[command(subcommand)]
    Submission(JmapSubmissionCommand),
    /// Manage JMAP vacation response.
    #[command(subcommand, visible_alias = "vacation")]
    VacationResponse(JmapVacationCommand),

    // The former form of `request`, a method-calls array printing
    // `{methodResponses}`, kept with its schema key.
    #[command(hide = true)]
    Query(JmapQueryCommand),
}

impl JmapCommand {
    /// Runs the subcommand against the account's JMAP client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        match self {
            Self::Mailbox(cmd) => cmd.execute(printer, account, client),
            Self::Email(cmd) => cmd.execute(printer, account, client),
            Self::Thread(cmd) => cmd.execute(printer, account, client),
            Self::Identity(cmd) => cmd.execute(printer, account, client),
            Self::Submission(cmd) => cmd.execute(printer, account, client),
            Self::VacationResponse(cmd) => cmd.execute(printer, account, client),
            Self::Request(cmd) => cmd.execute(printer, client),
            Self::Query(cmd) => cmd.execute(printer, client),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use crate::{cli::Cli, json_schema::registry};

    /// `request` names the escape hatch of an HTTP API across the family,
    /// taking one whole request object and printing `{response}`; `query`,
    /// its former form, still takes a method-calls array, hidden, its
    /// schema kept.
    #[test]
    fn the_raw_request_is_jmap_request() {
        let mut cli = Cli::command();
        let help = cli
            .find_subcommand_mut("jmap")
            .unwrap()
            .render_help()
            .to_string();
        let listed = |name: &str| {
            help.lines()
                .any(|line| line.trim_start().starts_with(&format!("{name} ")))
        };
        assert!(listed("request"), "{help}");
        assert!(!listed("query"), "{help}");
        assert!(help.contains("Send a raw JMAP request"), "{help}");

        let request = ["himalaya", "jmap", "request", r#"{"methodCalls": []}"#];
        assert!(Cli::try_parse_from(request).is_ok());
        let query = ["himalaya", "jmap", "query", "[]", "--using", "urn:x"];
        assert!(Cli::try_parse_from(query).is_ok());
        let request = ["himalaya", "jmap", "request", "{}", "--using", "urn:x"];
        assert!(Cli::try_parse_from(request).is_err());

        let schemas = registry().schemas;
        let request = &schemas["himalaya-jmap-request"];
        assert!(request.to_string().contains("response"), "{request}");
        let query = &schemas["himalaya-jmap-query"];
        assert!(query.to_string().contains("methodResponses"), "{query}");
    }
}
