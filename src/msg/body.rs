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
#[derive(Clone, Serialize, Debug)]
pub struct Body(String);

impl Body {
    pub fn new(content: &str) -> Self {
        Self(content.to_string())
    }

    pub fn new_with_string(content: String) -> Self {
        Self(content)
    }

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
impl From<&str> for Body {
    fn from(string: &str) -> Self {
        Self(string.to_string())
    }
}

impl From<String> for Body {
    fn from(string: String) -> Self {
        Self(string)
    }
}
