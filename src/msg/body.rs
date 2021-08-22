use error_chain::error_chain;

use std::fmt;

use serde::Serialize;

// == Macros ==
error_chain! {
    foreign_links {
        ParseContentType(lettre::message::header::ContentTypeErr);
    }
}

// == Structs ==
/// This struct represents the body/content of a msg. For example:
///
/// ```text
/// Dear Mr. Boss,
/// I like rust. It's an awesome language. *Change my mind*....
///
/// Sincerely
/// ```
///
/// This part of the msg/msg would be stored in this struct.
#[derive(Clone, Serialize, Debug, PartialEq, Eq)]
pub struct Body {
    /// The text version of a body (if available)
    pub text: Option<String>,

    /// The html version of a body (if available)
    pub html: Option<String>,
}

impl Body {
    /// Returns a new instance of `Body` without any attributes set. (Same as `Body::default()`)
    ///
    /// # Example
    /// ```rust
    /// use himalaya::msg::body::Body;
    ///
    /// fn main() {
    ///     let body = Body::new();
    ///
    ///     let expected_body = Body {
    ///         text: None,
    ///         html: None,
    ///     };
    ///     
    ///     assert_eq!(body, expected_body);
    /// }
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a new instance of `Body` with `text` set.
    ///
    /// # Example
    /// ```rust
    /// use himalaya::msg::body::Body;
    ///
    /// fn main() {
    ///     let body = Body::new_with_text("Text body");
    ///
    ///     let expected_body = Body {
    ///         text: Some("Text body".to_string()),
    ///         html: None,
    ///     };
    ///
    ///     assert_eq!(body, expected_body);
    /// }
    /// ```
    pub fn new_with_text<S: ToString>(text: S) -> Self {
        Self {
            text: Some(text.to_string()),
            html: None,
        }
    }

    /// Returns a new instance of `Body` with `html` set.
    ///
    /// # Example
    /// ```rust
    /// use himalaya::msg::body::Body;
    ///
    /// fn main() {
    ///     let body = Body::new_with_html("Html body");
    ///
    ///     let expected_body = Body {
    ///         text: None,
    ///         html: Some("Html body".to_string()),
    ///     };
    ///
    ///     assert_eq!(body, expected_body);
    /// }
    /// ```
    pub fn new_with_html<S: ToString>(html: S) -> Self {
        Self {
            text: None,
            html: Some(html.to_string()),
        }
    }

    /// Returns a new isntance of `Body` with `text` and `html` set.
    ///
    /// # Example
    /// ```rust
    /// use himalaya::msg::body::Body;
    ///
    /// fn main() {
    ///     let body = Body::new_with_both("Text body", "Html body");
    ///
    ///     let expected_body = Body {
    ///         text: Some("Text body".to_string()),
    ///         html: Some("Html body".to_string()),
    ///     };
    ///
    ///     assert_eq!(body, expected_body);
    /// }
    /// ```
    pub fn new_with_both<S: ToString>(text: S, html: S) -> Self {
        Self {
            text: Some(text.to_string()),
            html: Some(html.to_string()),
        }
    }
}

// == Traits ==
impl Default for Body {
    fn default() -> Self {
        Self {
            text: None,
            html: None,
        }
    }
}

impl fmt::Display for Body {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let content = if let Some(text) = self.text.clone() {
            text
        } else if let Some(html) = self.html.clone() {
            html
        } else {
            String::new()
        };

        write!(formatter, "{}", content)
    }
}
