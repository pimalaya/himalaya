//! Module related to message attachment CLI.
//!
//! This module provides arguments related to message attachment.

use clap::{App, Arg, SubCommand};

use crate::domain::msg;

/// Message attachment subcommands.
pub(crate) fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("attachments")
        .aliases(&["attachment", "att", "a"])
        .about("Downloads all message attachments")
        .arg(msg::arg::uid_arg())]
}

/// Message attachment path argument.
pub(crate) fn path_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("attachments")
        .help("Adds attachment to the message")
        .short("a")
        .long("attachment")
        .value_name("PATH")
        .multiple(true)
}
