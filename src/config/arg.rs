//! Module related to config CLI.
//!
//! This module provides arguments related to config.

use clap::Arg;

/// Config arguments.
pub fn args<'a>() -> Vec<Arg<'a, 'a>> {
    vec![
        Arg::with_name("config")
            .long("config")
            .short("c")
            .help("Forces a specific config path")
            .value_name("PATH"),
        Arg::with_name("account")
            .long("account")
            .short("a")
            .help("Selects a specific account")
            .value_name("NAME"),
    ]
}
