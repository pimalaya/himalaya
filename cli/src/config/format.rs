use serde::Deserialize;

/// Represents the text/plain format as defined in the [RFC2646]. The
/// format is then used by the table system to adjust the way it is
/// rendered.
///
/// [RFC2646]: https://www.ietf.org/rfc/rfc2646.txt
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(tag = "type", content = "width", rename_all = "lowercase")]
pub enum Format {
    // Forces the content width with a fixed amount of pixels.
    Fixed(usize),
    // Makes the content fit the terminal.
    Auto,
    // Does not restrict the content.
    Flowed,
}

impl Default for Format {
    fn default() -> Self {
        Self::Auto
    }
}
