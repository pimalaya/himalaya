use clap::Parser;

/// The envelope id argument parser.
#[derive(Debug, Parser)]
pub struct EnvelopeIdArg {
    /// The envelope id.
    #[arg(value_name = "ID", required = true)]
    pub id: usize,
}

/// The envelopes ids arguments parser.
#[derive(Debug, Parser)]
pub struct EnvelopeIdsArgs {
    /// The list of envelopes ids.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<usize>,
}
