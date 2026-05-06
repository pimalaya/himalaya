use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

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
    Get(JmapEmailGetCommand),
    Query(JmapEmailQueryCommand),
    Read(JmapEmailReadCommand),
    #[command(alias = "edit")]
    Update(JmapEmailUpdateCommand),
    #[command(aliases = ["remove", "rm"])]
    Delete(JmapEmailDestroyCommand),
    Copy(JmapEmailCopyCommand),
    Export(JmapEmailExportCommand),
    Import(JmapEmailImportCommand),
    Parse(JmapEmailParseCommand),
}

impl JmapEmailCommand {
    pub fn execute(self, printer: &mut impl Printer, client: JmapClient) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Query(cmd) => cmd.execute(printer, client),
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
