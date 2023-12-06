use clap::Parser;

/// The table max width argument parser
#[derive(Debug, Parser)]
pub struct MaxTableWidthFlag {
    /// The maximum width the table should not exceed
    #[arg(long, short = 'w', value_name = "PIXELS")]
    pub max_width: Option<usize>,
}
