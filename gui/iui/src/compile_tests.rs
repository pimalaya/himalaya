//! Examples of unsound code that IUI statically prevents from compiling.
//!
//! Here, we attempt to use-after-free some callbacks.
//!
//! ```compile_fail
//! let ui = iui::UI::init().unwrap();
//!
//! {
//!     let v = vec![1, 2, 3, 4];
//!     ui.queue_main(|| {
//!         for i in &v {
//!             println!("{}", i);
//!         }
//!     });
//! }
//! ```
//!
//! This one is OK, because it moves the `Vec` into the closure's scope.
//! ```no_run
//! let ev = iui::UI::init().unwrap();
//!
//! let v = vec![1, 2, 3, 4];
//! ev.on_should_quit(move || {
//!     for i in &v {
//!         println!("{}", i);
//!     }
//! });
//!
//! ev.quit();
//! ev.main();
//! ```
//!
//! This one tries to use a reference to a string that is dropped out of scope.
//! ```compile_fail
//! # use iui::prelude::*;
//! # use iui::controls::{Button};
//! let ui = UI::init().unwrap();
//! let mut win = Window::new(&ui, "Test App", 200, 200, WindowType::NoMenubar);
//! let mut button = Button::new(&ui, "Button");
//!
//! {
//!     let s = String::from("Whatever!");
//!     let callback =  |b: &mut Button| { println!("{}", s)};
//!     button.on_clicked(&ui, callback);
//! }
//!
//! win.set_child(&ui, button);
//! ```
//!
//! Here we try to use-after-free data in the on-tick callback.
//!
//! ```compile_fail
//! # use iui::prelude::*;
//! # use iui::controls::{Button};
//! let ui = UI::init().unwrap();
//! let mut ev = ui.event_loop();
//! let win = Window::new(&ui, "Test App", 200, 200, WindowType::NoMenubar);
//!
//! {
//!     let s = String::from("Whatever!");
//!     let callback =  || { println!("{}", s) };
//!     ev.on_tick(&ui, callback);
//! }
//!
//! ev.next_tick(&ui);
//! ui.quit();
//! ev.next_tick(&ui);
//! ```
