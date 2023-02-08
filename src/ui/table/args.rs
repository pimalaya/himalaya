use clap::{Arg, ArgMatches};

const ARG_MAX_TABLE_WIDTH: &str = "max-table-width";

pub(crate) type MaxTableWidth = Option<usize>;

/// Represents the max table width argument.
pub fn max_width() -> Arg {
    Arg::new(ARG_MAX_TABLE_WIDTH)
        .help("Defines a maximum width for the table")
        .long("max-width")
        .short('w')
        .value_name("INT")
}

/// Represents the max table width argument parser.
pub fn parse_max_width(matches: &ArgMatches) -> Option<usize> {
    matches
        .get_one::<String>(ARG_MAX_TABLE_WIDTH)
        .and_then(|s| s.parse().ok())
}
