//! `iui`, the `i`mproved `u`ser `i`nterface crate, is a **simple** (about 4 kLOC of Rust), **small** (about 800kb, including `libui`), **easy to distribute** (one shared library) GUI library, providing a **Rusty** user interface library that binds to **native APIs** via the [libui](https://github.com/andlabs/libui) and the `ui-sys` bindings crate.
//! `iui` wraps native retained mode GUI libraries, like Win32API on Windows, Cocoa on Mac OS X, and GTK+ on Linux and elsewhere. Thus all `iui` apps have a native look and feel and start from a highly performant base which is well integegrated with the native ecosystem on each platform. Because it implements only the least common subset of these platform APIs, your apps will work on all platforms and won't have significant behavioral inconsistencies, with no additional effort on your part.
//!
//! To use the library, add the following to your `Cargo.toml`:
//!
//! ```toml
//! "iui" = "0.3"
//! ```
//!
//! To build a GUI app with `iui`, you must:
//! 1. create a [`UI`](https://docs.rs/iui/*/iui/struct.UI.html#method.init) handle, initializing the UI library and guarding against memory unsafety
//! 1. make a [window](https://docs.rs/iui/*/iui/controls/struct.Window.html), or a few, with title and platform-native decorations, into which your app will be drawn
//! 1. add all your [controls](https://docs.rs/iui/*/iui/controls/index.html), like buttons and text inputs, laid out with both axial and grid layout options
//! 1. implement some [callbacks](https://docs.rs/iui/*/iui/controls/struct.Button.html#method.on_clicked) for user input, taking full advantage of Rust's concurrency protections
//! 1. call [`UI::main`](https://docs.rs/iui/*/iui/struct.UI.html#method.main), or take control over the event processing with an [`EventLoop`](https://docs.rs/iui/*/iui/struct.EventLoop.html), and voíla! A GUI!
//!
//! For code examples, see the [examples](https://github.com/rust-native-ui/libui-rs/blob/trunk/iui/examples/)
//! directory.

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate regex;
extern crate ui_sys;

mod callback_helpers;
mod compile_tests;
pub mod controls;
pub mod draw;
mod error;
mod ffi_tools;
pub mod menus;
pub mod str_tools;
mod ui;

pub use error::UIError;
pub use ui::{EventLoop, UI};

/// Common imports are packaged into this module. It's meant to be glob-imported: `use iui::prelude::*`.
pub mod prelude {
    pub use controls::LayoutStrategy;
    pub use controls::{NumericEntry, TextEntry};
    pub use controls::{Window, WindowType};
    pub use ui::UI;
}
