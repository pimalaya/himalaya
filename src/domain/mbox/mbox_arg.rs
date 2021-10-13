//! Mailbox CLI module.
//!
//! This module provides subcommands, arguments and a command matcher related to the mailbox
//! domain.

use anyhow::Result;
use clap;
use log::trace;

/// Represents the mailbox commands.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Cmd {
    /// Represents the list mailboxes command.
    List,
}

/// Defines the mailbox command matcher.
pub(crate) fn matches(m: &clap::ArgMatches) -> Result<Option<Cmd>> {
    if let Some(_) = m.subcommand_matches("mailboxes") {
        trace!("mailboxes subcommand matched");
        return Ok(Some(Cmd::List));
    }

    Ok(None)
}

/// Contains mailbox subcommands.
pub(crate) fn subcmds<'a>() -> Vec<clap::App<'a, 'a>> {
    vec![clap::SubCommand::with_name("mailboxes")
        .aliases(&["mailbox", "mboxes", "mbox", "mb", "m"])
        .about("Lists mailboxes")]
}

/// Defines the source mailbox argument.
pub(crate) fn source_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("mbox-source")
        .short("m")
        .long("mailbox")
        .help("Specifies the source mailbox")
        .value_name("SOURCE")
        .default_value("INBOX")
}

/// Defines the target mailbox argument.
pub(crate) fn target_arg<'a>() -> clap::Arg<'a, 'a> {
    clap::Arg::with_name("mbox-target")
        .help("Specifies the targetted mailbox")
        .value_name("TARGET")
        .required(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_match_cmds() {
        let arg = clap::App::new("himalaya")
            .subcommands(subcmds())
            .get_matches_from(&["himalaya", "mailboxes"]);

        assert_eq!(Some(Cmd::List), matches(&arg).unwrap());
    }

    #[test]
    fn it_should_match_aliases() {
        macro_rules! get_matches_from {
            ($alias:expr) => {
                clap::App::new("himalaya")
                    .subcommands(subcmds())
                    .get_matches_from(&["himalaya", $alias])
                    .subcommand_name()
            };
        }

        assert_eq!(Some("mailboxes"), get_matches_from!["mailboxes"]);
        assert_eq!(Some("mailboxes"), get_matches_from!["mboxes"]);
        assert_eq!(Some("mailboxes"), get_matches_from!["mbox"]);
        assert_eq!(Some("mailboxes"), get_matches_from!["mb"]);
        assert_eq!(Some("mailboxes"), get_matches_from!["m"]);
    }

    #[test]
    fn it_should_match_source_arg() {
        macro_rules! get_matches_from {
            ($($arg:expr),*) => {
                clap::App::new("himalaya")
                    .arg(source_arg())
                    .get_matches_from(&["himalaya", $($arg,)*])
            };
        }

        let app = get_matches_from![];
        assert_eq!(Some("INBOX"), app.value_of("mbox-source"));

        let app = get_matches_from!["-m", "SOURCE"];
        assert_eq!(Some("SOURCE"), app.value_of("mbox-source"));

        let app = get_matches_from!["--mailbox", "SOURCE"];
        assert_eq!(Some("SOURCE"), app.value_of("mbox-source"));
    }

    #[test]
    fn it_should_match_target_arg() {
        macro_rules! get_matches_from {
            ($($arg:expr),*) => {
                clap::App::new("himalaya")
                    .arg(target_arg())
                    .get_matches_from_safe(&["himalaya", $($arg,)*])
            };
        }

        let app = get_matches_from![];
        assert_eq!(
            clap::ErrorKind::MissingRequiredArgument,
            app.unwrap_err().kind
        );

        let app = get_matches_from!["TARGET"];
        assert_eq!(Some("TARGET"), app.unwrap().value_of("mbox-target"));
    }
}
