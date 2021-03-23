use clap::Arg;

pub fn output_args<'a>() -> Vec<Arg<'a, 'a>> {
    vec![Arg::with_name("output")
        .long("output")
        .short("o")
        .help("Defines the output format")
        .value_name("STRING")
        .possible_values(&["plain", "json"])
        .default_value("plain")]
}
