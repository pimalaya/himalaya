use clap::{self, App, Arg, ArgMatches, Shell, SubCommand};
use error_chain::error_chain;
use log::debug;
use std::io;

error_chain! {}

pub fn completion_subcmds<'c>() -> Vec<App<'c, 'c>> {
    vec![SubCommand::with_name("completion")
        .about("Generates the completion script for the given shell")
        .args(&[Arg::with_name("shell")
            .possible_values(&["bash", "zsh", "fish"])
            .required(true)])]
}

pub fn completion_matches(mut app: App, matches: &ArgMatches) -> Result<bool> {
    if let Some(matches) = matches.subcommand_matches("completion") {
        debug!("[completion::cli::matches] completion command matched");
        let shell = match matches.value_of("shell").unwrap() {
            "fish" => Shell::Fish,
            "zsh" => Shell::Zsh,
            "bash" | _ => Shell::Bash,
        };
        debug!("[completion::cli::matches] shell: {}", shell);
        app.gen_completions_to("himalaya", shell, &mut io::stdout());
        return Ok(true);
    };

    debug!("[completion::cli::matches] nothing matched");
    Ok(false)
}
