use clap::Parser;

/// The reply to all argument parser.
#[derive(Debug, Parser)]
pub struct MessageReplyAllArg {
    /// Reply to all recipients.
    ///
    /// This argument will add all recipients for the To and Cc
    /// headers.
    #[arg(long, short = 'A')]
    pub all: bool,
}
