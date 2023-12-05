use clap::{value_parser, Parser};
use clap_complete::Shell;

/// Print completion script for the given shell to stdout
#[derive(Debug, Parser)]
pub struct Generate {
    /// Shell that completion script should be generated for
    #[arg(value_parser = value_parser!(Shell))]
    pub shell: Shell,
}
