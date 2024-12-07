mod forward;
mod reply;
mod save;
mod send;
mod write;

use clap::Subcommand;
use color_eyre::Result;
use pimalaya_tui::terminal::cli::printer::Printer;

use crate::config::TomlConfig;

use self::{
    forward::TemplateForwardCommand, reply::TemplateReplyCommand, save::TemplateSaveCommand,
    send::TemplateSendCommand, write::TemplateWriteCommand,
};

/// Generate, save and send message templates.
///
/// A template is an editable version of a message (headers +
/// body). It uses a specific language called MML that allows you to
/// attach file or encrypt content. This subcommand allows you manage
/// them.
///
/// Learn more about MML at: <https://crates.io/crates/mml-lib>.
#[derive(Debug, Subcommand)]
pub enum TemplateSubcommand {
    #[command(aliases = ["add", "create", "new", "compose"])]
    Write(TemplateWriteCommand),

    #[command(arg_required_else_help = true)]
    Reply(TemplateReplyCommand),

    #[command(arg_required_else_help = true)]
    #[command(alias = "fwd")]
    Forward(TemplateForwardCommand),

    #[command()]
    Save(TemplateSaveCommand),

    #[command()]
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
