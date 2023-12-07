use clap::Parser;

/// The envelopes ids arguments parser
#[derive(Debug, Parser)]
pub struct EnvelopeIdsArgs {
    /// The list of envelopes ids
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<usize>,
}
