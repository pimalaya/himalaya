pub mod body;

use clap::Parser;

/// The raw template argument parser.
#[derive(Debug, Parser)]
pub struct TemplateRawArg {
    /// The raw template, including headers and MML body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "template_raw", value_name = "TEMPLATE")]
    pub raw: Vec<String>,
}

impl TemplateRawArg {
    pub fn raw(self) -> String {
        self.raw.join(" ").replace("\r", "")
    }
}
