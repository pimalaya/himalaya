pub mod forward;
pub mod reply;
pub mod save;
pub mod send;
pub mod write;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::{
    forward::TemplateForwardCommand, reply::TemplateReplyCommand, save::TemplateSaveCommand,
    send::TemplateSendCommand, write::TemplateWriteCommand,
};

/// Subcommand to manage templates
#[derive(Debug, Subcommand)]
pub enum TemplateSubcommand {
    /// Write a new template
    #[command(alias = "new", alias = "compose")]
    Write(TemplateWriteCommand),

    /// Reply to a template
    #[command()]
    Reply(TemplateReplyCommand),

    /// Generate a template for forwarding an email
    #[command(alias = "fwd")]
    Forward(TemplateForwardCommand),

    /// Save a template to a folder
    #[command(arg_required_else_help = true)]
    #[command(alias = "add", alias = "create")]
    Save(TemplateSaveCommand),

    /// Send a template
    #[command(arg_required_else_help = true)]
    Send(TemplateSendCommand),
}

impl TemplateSubcommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::Write(cmd) => cmd.execute(printer, config).await,
            Self::Reply(cmd) => cmd.execute(printer, config).await,
            Self::Forward(cmd) => cmd.execute(printer, config).await,
            Self::Save(cmd) => cmd.execute(printer, config).await,
            Self::Send(cmd) => cmd.execute(printer, config).await,
        }
    }
}
