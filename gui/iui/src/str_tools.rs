//! Tools for making platform-independent string handling work properly

use regex::{Regex, RegexBuilder};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Replaces every occurrance of `"\r\n"` with a single newline `\n`, without collapsing
/// newlines.
pub fn strip_dual_endings(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/// Replaces every occurrance of `"\n"` not followed by `"\r"` with `"\r\n"`.
pub fn insert_dual_endings(s: &str) -> String {
    lazy_static! {
        //static ref RE: Regex = Regex::new("([^\r])\n").expect("Could not compile regex");
        static ref RE: Regex = RegexBuilder::new("\r\n|\n")
                                             .multi_line(true)
                                             .build()
                                             .expect("Could not compile regex");
    }
    RE.replace_all(s, "\r\n").to_string()
}

/// Converts a &str to a CString, using either LF or CRLF as appropriate.
///
/// # Panics
/// Panics if it isn't possible to create a CString from the given string.
pub fn to_toolkit_string(s: &str) -> CString {
    let data = if cfg!(windows) {
        insert_dual_endings(s).as_bytes().to_vec()
    } else {
        s.as_bytes().to_vec()
    };
    CString::new(data).expect(&format!("Failed to create CString from {}", s))
}

/// Converts a `*mut c_char` to a String guaranteed to use LF line endings.
///
/// # Unsafety
/// Has the same unsafety as [CStr::from_ptr](https://doc.rust-lang.org/std/ffi/struct.CStr.html#method.from_ptr).
pub unsafe fn from_toolkit_string(c: *mut c_char) -> String {
    CStr::from_ptr(c).to_string_lossy().into_owned()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn strip_dual_endings_to_single() {
        assert_eq!(
            strip_dual_endings("Line 1\r\nLine 2\r\n"),
            "Line 1\nLine 2\n"
        );
    }

    #[test]
    fn insert_dual_endings_basic() {
        assert_eq!(
            insert_dual_endings("Line 1\nLine 2\n"),
            "Line 1\r\nLine 2\r\n"
        );
    }

    #[test]
    fn insert_dual_endings_nodupe() {
        assert_eq!(
            insert_dual_endings("Line 1\r\nLine 2\r\n"),
            "Line 1\r\nLine 2\r\n"
        );
    }

    #[test]
    fn test_toolkit_roundtripping() {
        let initial_string = "Here is some test data.\n\nMultiline!\n";
        let toolkit_string = to_toolkit_string(initial_string);
        let roundtripped_string = unsafe { from_toolkit_string(toolkit_string.into_raw()) };
        assert_eq!(initial_string, &roundtripped_string);
    }
}
