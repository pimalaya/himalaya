use std::io;
use clap::{self, App, ArgMatches, SubCommand};


pub fn complete_subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![
        SubCommand::with_name("bash-completions").about("Generate bash completions"),
        SubCommand::with_name("zsh-completions").about("Generate zsh completions"),
        SubCommand::with_name("fish-completions").about("Generate fish completions")
    ]
}

pub fn complete_matches(mut app: App, matches: &ArgMatches) -> Result<bool, io::Error> {
    if let Some(_) = matches.subcommand_matches("bash-completions") {
        app.gen_completions_to("himalaya", clap::Shell::Bash, &mut io::stdout());
        return Ok(true);
    }
    if let Some(_) = matches.subcommand_matches("zsh-completions") {
        app.gen_completions_to("himalaya", clap::Shell::Zsh, &mut io::stdout());
        return Ok(true);
    }
    if let Some(_) = matches.subcommand_matches("fish-completions") {
        app.gen_completions_to("himalaya", clap::Shell::Fish, &mut io::stdout());
        return Ok(true);
    }
    Ok(false)
}

