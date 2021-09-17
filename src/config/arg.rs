use clap::Arg;

/// Config argument.
pub fn path_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("config")
        .long("config")
        .short("c")
        .help("Forces a specific config path")
        .value_name("PATH")
}
