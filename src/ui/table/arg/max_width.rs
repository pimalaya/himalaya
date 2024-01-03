use clap::Parser;

/// The table max width argument parser.
#[derive(Debug, Default, Parser)]
pub struct TableMaxWidthFlag {
    /// The maximum width the table should not exceed.
    ///
    /// This argument will force the table not to exceed the given
    /// width in pixels. Columns may shrink with ellipsis in order to
    /// fit the width.
    #[arg(long, short = 'w', name = "table_max_width", value_name = "PIXELS")]
    pub max_width: Option<usize>,
}
