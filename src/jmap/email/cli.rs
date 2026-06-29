use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::jmap::{
    client::JmapClient,
    email::{
        copy::JmapEmailCopyCommand, delete::JmapEmailDestroyCommand,
        export::JmapEmailExportCommand, get::JmapEmailGetCommand, import::JmapEmailImportCommand,
        parse::JmapEmailParseCommand, query::JmapEmailQueryCommand, read::JmapEmailReadCommand,
        update::JmapEmailUpdateCommand,
    },
};

/// Manage JMAP emails.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum JmapEmailCommand {
    /// Get JMAP emails by ID (Email/get).
    Get(JmapEmailGetCommand),
    /// Query JMAP emails (Email/query + Email/get).
    Query(JmapEmailQueryCommand),
    /// Read the content of a JMAP email (Email/get with body).
    Read(JmapEmailReadCommand),
    /// Update JMAP emails via patch operations (Email/set).
    #[command(alias = "edit")]
    Update(JmapEmailUpdateCommand),
    /// Delete JMAP emails (Email/set destroy).
    #[command(aliases = ["remove", "rm"])]
    Delete(JmapEmailDestroyCommand),
    /// Copy JMAP emails from another account (Email/copy).
    Copy(JmapEmailCopyCommand),
    /// Export a raw RFC 5322 message to stdout (Email/get + blob download).
    Export(JmapEmailExportCommand),
    /// Import an RFC 5322 message into a mailbox (upload + Email/import).
    Import(JmapEmailImportCommand),
    /// Parse RFC 5322 message blobs without storing them (Email/parse).
    Parse(JmapEmailParseCommand),
}

impl JmapEmailCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::Query(cmd) => cmd.execute(printer, account, client),
            Self::Read(cmd) => cmd.execute(printer, client),
            Self::Update(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Export(cmd) => cmd.execute(printer, client),
            Self::Import(cmd) => cmd.execute(printer, client),
            Self::Parse(cmd) => cmd.execute(printer, client),
        }
    }
}
