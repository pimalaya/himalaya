//! This module provides arguments related to the user config.

use clap::{Arg, ArgMatches};

const ARG_CONFIG: &str = "config";

/// Represents the config file path argument. This argument allows the
/// user to customize the config file path.
pub fn global_args() -> impl IntoIterator<Item = Arg> {
    [Arg::new(ARG_CONFIG)
        .help("Override the configuration file path")
        .long_help(
            "Override the configuration file path

If the file under the given path does not exist, the wizard will propose to create it.",
        )
        .long("config")
        .short('c')
        .global(true)
        .value_name("path")]
}

/// Represents the config file path argument parser.
pub fn parse_global_arg(matches: &ArgMatches) -> Option<&str> {
    matches.get_one::<String>(ARG_CONFIG).map(String::as_str)
}
