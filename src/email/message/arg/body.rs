use std::ops::Deref;

use clap::Parser;

/// The raw message body argument parser
#[derive(Debug, Parser)]
pub struct BodyRawArg {
    /// Prefill the template with a custom body
    #[arg(raw = true, required = false)]
    #[arg(name = "body-raw", value_delimiter = ' ')]
    pub raw: Vec<String>,
}

impl BodyRawArg {
    pub fn raw(self) -> String {
        self.raw.join(" ").replace("\r", "").replace("\n", "\r\n")
    }
}

impl Deref for BodyRawArg {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}
