use clap::Arg;

pub fn output_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("output")
        .long("output")
        .short("o")
        .help("Defines the output format")
        .value_name("STRING")
        .possible_values(&["plain", "json"])
        .default_value("plain")
}
