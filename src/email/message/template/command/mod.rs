#[cfg(feature = "template-forward")]
pub mod forward;
#[cfg(feature = "template-reply")]
pub mod reply;
#[cfg(feature = "template-save")]
pub mod save;
#[cfg(feature = "template-send")]
pub mod send;
#[cfg(feature = "template-write")]
pub mod write;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

#[cfg(feature = "template-forward")]
use self::forward::TemplateForwardCommand;
#[cfg(feature = "template-reply")]
use self::reply::TemplateReplyCommand;
#[cfg(feature = "template-save")]
use self::save::TemplateSaveCommand;
#[cfg(feature = "template-send")]
use self::send::TemplateSendCommand;
#[cfg(feature = "template-write")]
use self::write::TemplateWriteCommand;

/// Manage templates.
///
/// A template is an editable version of a message (headers +
/// body). It uses a specific language called MML that allows you to
/// attach file or encrypt content. This subcommand allows you manage
/// them.
///
/// You can learn more about MML at
/// <https://crates.io/crates/mml-lib>.
#[derive(Debug, Subcommand)]
pub enum TemplateSubcommand {
    #[cfg(feature = "template-write")]
    #[command(aliases = ["add", "create", "new", "compose"])]
    Write(TemplateWriteCommand),

    #[cfg(feature = "template-reply")]
    #[command(arg_required_else_help = true)]
    Reply(TemplateReplyCommand),

    #[cfg(feature = "template-forward")]
    #[command(arg_required_else_help = true)]
    #[command(alias = "fwd")]
    Forward(TemplateForwardCommand),

    #[cfg(feature = "template-save")]
    #[command()]
    Save(TemplateSaveCommand),

    #[cfg(feature = "template-send")]
    #[command()]
    Send(TemplateSendCommand),
}

impl TemplateSubcommand {
    #[allow(unused)]
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            #[cfg(feature = "template-write")]
            Self::Write(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "template-reply")]
            Self::Reply(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "template-forward")]
            Self::Forward(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "template-save")]
            Self::Save(cmd) => cmd.execute(printer, config).await,
            #[cfg(feature = "template-send")]
            Self::Send(cmd) => cmd.execute(printer, config).await,
        }
    }
}
