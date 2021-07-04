// ===========
// Usages
// ===========
use error_chain::error_chain;

use std::ops::{Deref, DerefMut};
use std::fmt;

use serde::Serialize;

// ===========
// Macros
// ===========
error_chain!{
    foreign_links {
         ParseContentType(lettre::message::header::ContentTypeErr);
    }
}

// ============
// Structs
// ============
/// This struct represents the body/content of a mail/msg. For example:
///
/// ```text
/// Dear Mr. Boss,
/// I like rust. It's an awesome language. *Change my mind*....
///
/// Sincerely
/// ```
/// 
/// This part of the msg/mail would be stored in this struct.
#[derive(Clone, Serialize, Debug, PartialEq, Eq)]
pub struct Body(String);

impl Body {
    /// This function just returns a clone of it's content. If we use the
    /// example from above, than you'll get a clone of the whole text.
    ///
    /// # Example
    /// ```
    /// # use himalaya::msg::body::Body;
    /// # fn main() {
    /// let body = concat![
    ///     "Dear Mr. Boss,\n",
    ///     "I like rust. It's an awesome language. *Change my mind*....\n",
    ///     "\n",
    ///     "Sincerely",
    /// ];
    ///
    /// // create a new instance of `Body`
    /// let body_struct = Body::from(body);
    ///
    /// assert_eq!(body_struct.get_content(), body);
    /// # }
    /// ```
    pub fn get_content(&self) -> String {
        self.0.clone()
    }
}

// ===========
// Traits
// ===========
// ------------
// Commons
// ------------
impl Default for Body {
    fn default() -> Self {
        Self(String::new())
    }
}

impl fmt::Display for Body {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "{}", &self.0)
    }
}

impl Deref for Body {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Body {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// -----------
// From's
// -----------
/// Give in a `&str` to create a new instance of `Body`.
///
/// # Example
/// ```
/// # use himalaya::msg::body::Body;
/// # fn main() {
/// Body::from("An awesome string!");
/// # }
/// ```
impl From<&str> for Body {
    fn from(string: &str) -> Self {
        Self(string.to_string())
    }
}

/// Give in a [`String`] to create a new instance of `Body`.
///
/// # Example
/// ```
/// # use himalaya::msg::body::Body;
/// # fn main() {
/// let body_content = String::from("A awesome content.");
/// Body::from(body_content);
/// # }
/// ```
///
/// [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
impl From<String> for Body {
    fn from(string: String) -> Self {
        Self(string)
    }
}
