use clap::Parser;

/// The skip-editor argument parser.
#[derive(Debug, Parser)]
pub struct MessageYesArg {
    /// Skip the editor and send the message immediately.
    ///
    /// This argument requires --body to be set. When provided, the
    /// message is compiled and sent directly without opening the
    /// editor defined in $EDITOR.
    #[arg(long, short = 'y')]
    pub yes: bool,
}
