use clap::Arg;

/// Defines the max table width argument.
pub fn max_width<'a>() -> Arg<'a, 'a> {
    Arg::with_name("max-table-width")
        .help("Defines a maximum width for the table")
        .short("w")
        .long("max-width")
        .value_name("INT")
}
