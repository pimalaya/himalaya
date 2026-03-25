use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{
    account::JmapAccount,
    email::{
        copy::JmapEmailCopyCommand, delete::JmapEmailDestroyCommand, export::ExportEmailCommand,
        get::JmapEmailGetCommand, import::ImportEmailCommand, parse::ParseEmailCommand,
        query::JmapEmailQueryCommand, read::ReadEmailCommand, update::JmapEmailUpdateCommand,
    },
};

/// Manage JMAP emails.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum JmapEmailCommand {
    Get(JmapEmailGetCommand),
    Query(JmapEmailQueryCommand),
    Read(ReadEmailCommand),
    #[command(alias = "edit")]
    Update(JmapEmailUpdateCommand),
    #[command(aliases = ["remove", "rm"])]
    Delete(JmapEmailDestroyCommand),
    Copy(JmapEmailCopyCommand),
    Export(ExportEmailCommand),
    Import(ImportEmailCommand),
    Parse(ParseEmailCommand),
}

impl JmapEmailCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::Query(cmd) => cmd.execute(printer, account),
            Self::Read(cmd) => cmd.execute(printer, account),
            Self::Update(cmd) => cmd.execute(printer, account),
            Self::Delete(cmd) => cmd.execute(printer, account),
            Self::Copy(cmd) => cmd.execute(printer, account),
            Self::Export(cmd) => cmd.execute(printer, account),
            Self::Import(cmd) => cmd.execute(printer, account),
            Self::Parse(cmd) => cmd.execute(printer, account),
        }
    }
}
