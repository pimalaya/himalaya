use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::jmap::{
    account::JmapAccount,
    keyword::{add::AddKeywordCommand, remove::RemoveKeywordCommand, set::SetKeywordsCommand},
};

/// Manage JMAP email keywords (flags).
#[derive(Debug, Subcommand)]
pub enum KeywordCommand {
    Add(AddKeywordCommand),
    Remove(RemoveKeywordCommand),
    Set(SetKeywordsCommand),
}

impl KeywordCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, account),
            Self::Remove(cmd) => cmd.execute(printer, account),
            Self::Set(cmd) => cmd.execute(printer, account),
        }
    }
}
