use anyhow::{bail, Result};
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::shared::{
    client::EmailClient,
    messages::{output, runner},
};

/// Compose a new message by delegating to a user-defined composer.
///
/// Looks `<name>` up in `[message.composer.<name>]` and runs its
/// `command` via `sh -c`. With no `<name>`, falls back to the entry
/// flagged `default = true`. The escape hatch `--command "<sh>"`
/// lets you run an ad-hoc command without editing the config.
///
/// The composer takes the terminal: stdin is left empty (new
/// message — no source), stderr is inherited (composer prompts/
/// errors). The composer's stdout must be a valid RFC 5322 message,
/// which himalaya then routes through `--save` / `--send`, or to
/// stdout if neither is set.
#[derive(Debug, Parser)]
pub struct MessageComposeWithCommand {
    /// Name of an entry in `[message.composer.*]`. Optional — when
    /// omitted, the composer flagged `default = true` is used.
    #[arg(value_name = "NAME", conflicts_with = "command")]
    pub name: Option<String>,

    /// Ad-hoc shell command, mutually exclusive with `<name>`.
    /// Useful for trying the feature before editing the config.
    #[arg(long, value_name = "SHELL")]
    pub command: Option<String>,

    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,

    #[arg(long)]
    pub send: bool,
}

impl MessageComposeWithCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let command = match self.command.as_deref() {
            Some(cmd) => cmd.to_owned(),
            None => {
                runner::resolve_composer(&client.account.composer, self.name.as_deref())?.to_owned()
            }
        };

        let raw = runner::run(&command, &[])?;
        if raw.is_empty() {
            bail!("composer `{command}` produced no output");
        }

        output::route(printer, &mut client, raw, self.save.as_deref(), self.send)
    }
}
