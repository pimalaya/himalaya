//! This module provides arguments related to the user account config.

use clap::Arg;

/// Represents the user account name argument.
/// This argument allows the user to select a different account than the default one.
pub fn name_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("account")
        .long("account")
        .short("a")
        .help("Selects a specific account")
        .value_name("NAME")
}
