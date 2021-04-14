use clap::{self, App, ArgMatches, SubCommand};
use error_chain::error_chain;
use std::io;

error_chain! {}

pub fn completion_subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![
        SubCommand::with_name("bash-completions").about("Generates bash completions script"),
        SubCommand::with_name("zsh-completions").about("Generates zsh completions script"),
        SubCommand::with_name("fish-completions").about("Generates fish completions script"),
    ]
}

pub fn completion_matches(mut app: App, matches: &ArgMatches) -> Result<bool> {
    use clap::Shell::*;

    if matches.is_present("bash-completions") {
        app.gen_completions_to("himalaya", Bash, &mut io::stdout());
        return Ok(true);
    }

    if matches.is_present("zsh-completions") {
        app.gen_completions_to("himalaya", Zsh, &mut io::stdout());
        return Ok(true);
    }

    if matches.is_present("fish-completions") {
        app.gen_completions_to("himalaya", Fish, &mut io::stdout());
        return Ok(true);
    }

    Ok(false)
}
