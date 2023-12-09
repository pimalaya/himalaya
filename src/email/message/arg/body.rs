use clap::Parser;
use std::ops::Deref;

/// The raw message body argument parser.
#[derive(Debug, Parser)]
pub struct MessageRawBodyArg {
    /// Prefill the template with a custom body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "body_raw", value_name = "BODY")]
    pub raw: Vec<String>,
}

impl MessageRawBodyArg {
    pub fn raw(self) -> String {
        self.raw.join(" ").replace("\r", "").replace("\n", "\r\n")
    }
}

impl Deref for MessageRawBodyArg {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}
