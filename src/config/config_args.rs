//! This module provides arguments related to the user config.

use clap::Arg;

/// Represents the config path argument.
/// This argument allows the user to customize the config file path.
pub fn path_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("config")
        .long("config")
        .short("c")
        .help("Forces a specific config path")
        .value_name("PATH")
}
