use clap::Parser;

/// The reply to all argument parser
#[derive(Debug, Parser)]
pub struct MessageReplyAllArg {
    /// Reply to all recipients
    #[arg(long, short = 'A')]
    pub all: bool,
}
