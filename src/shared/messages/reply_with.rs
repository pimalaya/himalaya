use anyhow::{bail, Result};
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::shared::{
    client::EmailClient,
    messages::{output, runner},
};

/// Reply to a message by delegating to a user-defined composer.
///
/// Fetches the source message, then runs the named (or default)
/// composer with the source MIME piped on stdin. The composer must
/// consume stdin first if it wants user interaction — TUI composers
/// can re-open `/dev/tty` once stdin is drained (vim/less/fzf all do
/// this). The produced MIME is routed through `--save` / `--send`,
/// or stdout if neither is set.
#[derive(Debug, Parser)]
pub struct MessageReplyWithCommand {
    /// Identifier of the source message.
    #[arg(value_name = "ID")]
    pub id: String,

    /// Mailbox the source message lives in. Ignored for JMAP.
    #[arg(
        long = "mailbox",
        short = 'm',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub mailbox: String,

    #[arg(value_name = "NAME", conflicts_with = "command")]
    pub name: Option<String>,

    #[arg(long, value_name = "SHELL")]
    pub command: Option<String>,

    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,

    #[arg(long)]
    pub send: bool,
}

impl MessageReplyWithCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let source = client.get_message(&self.mailbox, &self.id)?;

        let command = match self.command.as_deref() {
            Some(cmd) => cmd.to_owned(),
            None => {
                runner::resolve_composer(&client.account.composer, self.name.as_deref())?.to_owned()
            }
        };

        let raw = runner::run(&command, &source)?;
        if raw.is_empty() {
            bail!("composer `{command}` produced no output");
        }

        output::route(printer, &mut client, raw, self.save.as_deref(), self.send)
    }
}
