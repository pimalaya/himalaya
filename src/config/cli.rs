use clap::Arg;

pub fn config_args<'a>() -> Vec<Arg<'a, 'a>> {
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
