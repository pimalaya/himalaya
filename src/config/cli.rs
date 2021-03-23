use clap::Arg;

pub fn account_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("account")
        .long("account")
        .short("a")
        .help("Selects a specific account")
        .value_name("STRING")
}
