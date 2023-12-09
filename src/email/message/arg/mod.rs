use clap::Parser;

pub mod body;
pub mod header;
pub mod reply;

/// The raw message argument parser.
#[derive(Debug, Parser)]
pub struct MessageRawArg {
    /// The raw message, including headers and body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "message_raw", value_name = "MESSAGE")]
    pub raw: Vec<String>,
}

impl MessageRawArg {
    pub fn raw(self) -> String {
        self.raw.join(" ").replace("\r", "").replace("\n", "\r\n")
    }
}
