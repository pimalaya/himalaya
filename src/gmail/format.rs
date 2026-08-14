//! Amount of message detail requested by the Gmail `get` commands.
//!
//! Gmail exposes the same `format` query parameter on `messages.get`,
//! `drafts.get` and `threads.get`, so the three commands share a single
//! [`ValueEnum`] instead of declaring one each.

use clap::ValueEnum;
use io_gmail::v1::rest::messages::GmailMessageFormat;

/// Amount of Gmail message detail to return (`format` query parameter).
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FormatArg {
    /// Identifiers and labels only, without headers or body.
    Minimal,
    /// The parsed payload: headers, MIME structure and bodies.
    #[default]
    Full,
    /// The whole message as raw RFC 5322 bytes.
    Raw,
    /// Headers only, narrowed down by the `--header` option.
    Metadata,
}

impl From<FormatArg> for GmailMessageFormat {
    fn from(arg: FormatArg) -> Self {
        match arg {
            FormatArg::Minimal => GmailMessageFormat::Minimal,
            FormatArg::Full => GmailMessageFormat::Full,
            FormatArg::Raw => GmailMessageFormat::Raw,
            FormatArg::Metadata => GmailMessageFormat::Metadata,
        }
    }
}
