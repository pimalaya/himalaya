use clap::Parser;
use std::ops::Deref;

/// The raw template body argument parser.
#[derive(Debug, Parser)]
pub struct TemplateRawBodyArg {
    /// Prefill the template with a custom body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "body-raw")]
    pub raw: Vec<String>,
}

impl TemplateRawBodyArg {
    pub fn raw(self) -> String {
        self.raw.join(" ").replace("\r", "")
    }
}

impl Deref for TemplateRawBodyArg {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}
