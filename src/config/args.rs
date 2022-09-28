//! This module provides arguments related to the user config.

use clap::{Arg, ArgMatches};

const ARG_CONFIG: &str = "config";

/// Represents the config file path argument. This argument allows the
/// user to customize the config file path.
pub fn arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_CONFIG)
        .long("config")
        .short("c")
        .help("Forces a specific config file path")
        .value_name("PATH")
}

/// Represents the config file path argument parser.
pub fn parse_arg<'a>(matches: &'a ArgMatches<'a>) -> Option<&'a str> {
    matches.value_of(ARG_CONFIG)
}
