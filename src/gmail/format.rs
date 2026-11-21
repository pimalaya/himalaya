//! # Gmail format
//!
//! How much message detail the Gmail `get` commands ask for, and how
//! their text forms render the snippet Gmail returns.
//!
//! Gmail exposes the same `format` query parameter on `messages.get`,
//! `drafts.get` and `threads.get`, so the three commands share one
//! [`ValueEnum`] rather than declare one each.

use clap::ValueEnum;
use io_gmail::v1::rest::messages::GmailMessageFormat;

/// Amount of Gmail message detail to return (`format` query parameter).
///
/// Defaults to `metadata`, which carries everything the commands print
/// without fetching bodies. Ask for `full` to get the MIME payload.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum FormatArg {
    /// Identifiers and labels only, without headers or body.
    Minimal,
    /// The parsed payload: headers, MIME structure and bodies.
    Full,
    /// The whole message as raw RFC 5322 bytes.
    Raw,
    /// Headers only, narrowed down by the `--header` option.
    #[default]
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

impl FormatArg {
    /// The query of a `get` at this format, with one `metadataHeaders` per
    /// header, which Gmail honours under the metadata format alone.
    pub fn query(self, headers: &[String]) -> Vec<(String, String)> {
        let format = match self {
            Self::Minimal => "minimal",
            Self::Full => "full",
            Self::Raw => "raw",
            Self::Metadata => "metadata",
        };
        let mut pairs = vec![(String::from("format"), String::from(format))];

        if let Self::Metadata = self {
            for header in headers {
                pairs.push((String::from("metadataHeaders"), header.clone()));
            }
        }

        pairs
    }
}

/// A Gmail snippet as plain text: Gmail escapes it as HTML and marks the
/// words a query matched with `<b>`, which a terminal would show as is.
pub fn snippet_text(snippet: &str) -> String {
    snippet
        .replace("<b>", "")
        .replace("</b>", "")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// The string at `key` of an item Gmail sent, empty when it has none.
pub fn str_of<'a>(item: &'a serde_json::Value, key: &str) -> &'a str {
    item.get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::gmail::format::snippet_text;

    #[test]
    fn a_snippet_reads_as_plain_text() {
        assert_eq!(
            snippet_text("Hello <b>pelican</b> &gt; it&#39;s &amp;lt; &quot;x&quot;"),
            "Hello pelican > it's &lt; \"x\""
        );
    }
}
