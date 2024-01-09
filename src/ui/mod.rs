pub mod choice;
pub mod editor;
pub(crate) mod prompt;
pub mod table;

use dialoguer::theme::ColorfulTheme;
use once_cell::sync::Lazy;

pub use self::table::*;

pub(crate) static THEME: Lazy<ColorfulTheme> = Lazy::new(ColorfulTheme::default);
