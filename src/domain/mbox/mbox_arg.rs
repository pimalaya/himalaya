//! Mailbox CLI module.
//!
//! This module provides subcommands, arguments and a command matcher related to the mailbox
//! domain.

use anyhow::Result;
use clap;

/// Represents the mailbox commands.
pub(crate) enum Command {
    /// Represents the list mailboxes command.
    List,
}

/// Defines the mailbox command matcher.
pub(crate) fn matches(m: &clap::ArgMatches) -> Result<Option<Command>> {
    if let Some(_) = m.subcommand_matches("mailboxes") {
        return Ok(Some(Command::List));
    }

    Ok(None)
}

/// Defines the source mailbox argument.
pub(crate) fn source_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("mailbox")
        .short("m")
        .long("mailbox")
        .help("Specifies the source mailbox")
        .value_name("SOURCE")
        .default_value("INBOX")
}

/// Defines the target mailbox argument.
pub(crate) fn target_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("target")
        .help("Specifies the targetted mailbox")
        .value_name("TARGET")
        .required(true)
}

/// Contains the mailbox subcommands.
pub(crate) fn subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![clap::SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "mb", "m"])
        .about("Lists all mailboxes")]
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! subcmd_alias {
        ($alias:expr) => {
            clap::App::new("himalaya")
                .subcommands(subcmds())
                .get_matches_from(&["himalaya", $alias])
                .subcommand_name()
        };
    }

    #[test]
    fn test_subcmds_aliases() {
        assert_eq!(Some("mailboxes"), subcmd_alias!("mailboxes"));
        assert_eq!(Some("mailboxes"), subcmd_alias!("mboxes"));
        assert_eq!(Some("mailboxes"), subcmd_alias!("mbox"));
        assert_eq!(Some("mailboxes"), subcmd_alias!("mb"));
        assert_eq!(Some("mailboxes"), subcmd_alias!("m"));
    }
}
