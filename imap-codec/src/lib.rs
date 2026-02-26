//! # IMAP protocol library
//!
//! imap-codec provides complete and detailed parsing and construction of [IMAP4rev1] commands and responses.
//! It is based on [imap-types] and extends it with parsing support using [nom].
//!
//! The main codecs are
//! [`GreetingCodec`] (to parse the first message from a server),
//! [`CommandCodec`] (to parse commands from a client), and
//! [`ResponseCodec`] (to parse responses or results from a server).
//!
//! Note that IMAP traces are not guaranteed to be UTF-8.
//! Thus, be careful when using code like `from_utf8(...)`.
//!
//! ## Decoding
//!
//! Decoding is provided through the [`Decoder`](`crate::decode::Decoder`) trait.
//! Every parser takes an input (`&[u8]`) and produces a remainder and a parsed value.
//!
//! **Note:** Decoding IMAP traces is more elaborate than it seems on a first glance.
//! Please consult the [`decode`](`crate::decode`) module documentation to learn how to handle real-world decoding.
//!
//! ### Example
//!
//! ```rust
//! use imap_codec::{
//!     GreetingCodec,
//!     decode::Decoder,
//!     imap_types::{
//!         core::Text,
//!         response::{Code, Greeting, GreetingKind},
//!     },
//! };
//!
//! let (remaining, greeting) = GreetingCodec::default()
//!     .decode(b"* OK [ALERT] Hello, World!\r\n<remaining>")
//!     .unwrap();
//!
//! assert_eq!(
//!     greeting,
//!     Greeting {
//!         kind: GreetingKind::Ok,
//!         code: Some(Code::Alert),
//!         text: Text::try_from("Hello, World!").unwrap(),
//!     }
//! );
//! assert_eq!(remaining, &b"<remaining>"[..])
//! ```
//!
//! ## Encoding
//!
//! Encoding is provided through the [`Encoder`](`crate::encode::Encoder`) trait.
//!
//! **Note:** Encoding IMAP traces is more elaborate than it seems on a first glance.
//! Please consult the [`encode`](`crate::encode`) module documentation to learn how to handle real-world encoding.
//!
//! ### Example
//!
//! ```rust
//! use imap_codec::{
//!     GreetingCodec,
//!     encode::Encoder,
//!     imap_types::{
//!         core::Text,
//!         response::{Code, Greeting, GreetingKind},
//!     },
//! };
//!
//! let greeting = Greeting {
//!     kind: GreetingKind::Ok,
//!     code: Some(Code::Alert),
//!     text: Text::try_from("Hello, World!").unwrap(),
//! };
//!
//! let bytes = GreetingCodec::default().encode(&greeting).dump();
//!
//! assert_eq!(bytes, &b"* OK [ALERT] Hello, World!\r\n"[..]);
//! ```
//!
//! ## Features
//!
//! imap-codec forwards many features to imap-types. See [imap-types features] for a comprehensive list.
//!
//! In addition, imap-codec defines the following features:
//!
//! | Feature               | Description                    | Enabled by default |
//! |-----------------------|--------------------------------|--------------------|
//! | quirk_crlf_relaxed    | Make `\r` in `\r\n` optional.  | No                 |
//! | quirk_rectify_numbers | Rectify (invalid) numbers.     | No                 |
//! | quirk_missing_text    | Rectify missing `text` element.| No                 |
//!
//! ## Quirks
//!
//! Features starting with `quirk_` are used to cope with existing interoperability issues.
//! Unfortunately, we already observed some standard violations, such as, negative numbers, and missing syntax elements.
//! Our policy is as follows: If we see an interoperability issue, we file an issue in the corresponding implementation.
//! If, for any reason, the issue cannot be fixed, *and* the implementation is "important enough", e.g.,  because a user of
//! imap-codec can't otherwise access their emails, we may add a `quirk_` feature to quickly resolve the problem.
//! Of course, imap-codec should never violate the IMAP standard itself. So, we need to do this carefully.
//!
//! [imap-types]: https://docs.rs/imap-types/latest/imap_types
//! [imap-types features]: https://docs.rs/imap-types/latest/imap_types/#features
//! [IMAP4rev1]: https://tools.ietf.org/html/rfc3501
//! [parse_command]: https://github.com/duesee/imap-codec/blob/main/examples/parse_command.rs

// TODO(#660)
#![allow(unknown_lints)]
#![allow(mismatched_lifetime_syntaxes)]
#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Test examples from repository root README.
#[doc = include_str!("../../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctestsRoot;

// Test examples from imap-codec's README.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

mod auth;
mod body;
mod codec;
mod command;
mod core;
mod datetime;
mod envelope;
mod extensions;
mod fetch;
mod flag;
mod mailbox;
mod response;
mod search;
mod sequence;
mod status;
#[cfg(test)]
mod testing;

pub mod fragmentizer;
#[cfg(feature = "fuzz")]
pub mod fuzz {
    pub use crate::core::fuzz_tag_imap;
}

pub use codec::*;
// Re-export.
pub use imap_types;
