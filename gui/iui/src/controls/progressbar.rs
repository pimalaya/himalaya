use super::Control;
use std::mem;
use ui::UI;
use ui_sys::{self, uiControl, uiProgressBar};

/// An enum representing the value of a `ProgressBar`.
///
/// # Values
///
/// A `ProgressBarValue` can be either `Determinate`, a number from 0 up to 100, or
/// `Indeterminate`, representing a process that is still in progress but has no
/// completeness metric availble.
///
/// # Conversions
///
/// A `ProgressBarValue` can be made from a `u32` or an `Option<u32>`, and the relevant functions
/// take a type that is generic over this behavior, so it's easy to set the progress of a bar.
///
/// ```
/// # use iui::prelude::*;
/// # use iui::controls::{ProgressBar, ProgressBarValue};
/// # let ui = UI::init().unwrap();
/// # if cfg!(target_os = "macos") { return; }
/// # let mut window = Window::new(&ui, "Test Window", 0, 0, WindowType::NoMenubar);
/// let mut progressbar = ProgressBar::indeterminate(&ui);
/// progressbar.set_value(&ui, 54);
///
/// // Perhaps this is the result of some fallible progress-checking function.
/// let maybe_progress: Option<u32> = None;
/// progressbar.set_value(&ui, maybe_progress);
///
/// // And of course, you can always set it by hand.
/// progressbar.set_value(&ui, ProgressBarValue::Indeterminate);
/// # window.set_child(&ui, progressbar);
/// # ui.quit();
/// # ui.main();
/// ```
pub enum ProgressBarValue {
    /// Represents a set, consistent percentage of the bar to be filled
    ///
    /// The value should be in the range 0..=100, and will be capped at 100
    /// by ProgressBar::set_value if it is larger.
    Determinate(u32),
    /// Represents an indeterminate value of the progress bar, useful
    /// if you don't know how much of the task being represented is completed.
    Indeterminate,
}

impl From<u32> for ProgressBarValue {
    fn from(value: u32) -> ProgressBarValue {
        if value <= 100 {
            ProgressBarValue::Determinate(value)
        } else {
            ProgressBarValue::Determinate(100)
        }
    }
}

impl From<Option<u32>> for ProgressBarValue {
    fn from(value: Option<u32>) -> ProgressBarValue {
        match value {
            Some(v) => v.into(),
            None => ProgressBarValue::Indeterminate,
        }
    }
}

define_control! {
  /// A bar that fills up with a set percentage, used to show completion of a
  ///
  /// # Values
  /// A `ProgressBar` can be either determinate or indeterminate. See [`ProgressBarValue`]
  /// for an explanation of the differences.
  ///
  /// [`ProgressBarValue`]: enum.ProgressBarValue.html
  rust_type: ProgressBar,
  sys_type: uiProgressBar,
}

impl ProgressBar {
    /// Create a new progress bar with a value of 0
    pub fn new() -> ProgressBar {
        unsafe { ProgressBar::from_raw(ui_sys::uiNewProgressBar()) }
    }

    /// Create a new indeterminate progress bar
    pub fn indeterminate(ctx: &UI) -> ProgressBar {
        let mut pb = ProgressBar::new();
        pb.set_value(ctx, ProgressBarValue::Indeterminate);
        pb
    }

    /// Set the value of the progress bar. See [`ProgressBarValue`] for the values that can be passed in.
    /// [`ProgressBarValue`]: enum.ProgressBarValue.html
    pub fn set_value<V: Into<ProgressBarValue>>(&mut self, _ctx: &UI, value: V) {
        let sys_value = match value.into() {
            ProgressBarValue::Determinate(value) => {
                let value = if value > 100 { 100 } else { value };
                value as i32
            }
            ProgressBarValue::Indeterminate => -1,
        };
        unsafe { ui_sys::uiProgressBarSetValue(self.uiProgressBar, sys_value) }
    }

    /// Get the value of the progress bar
    pub fn value(&self, _ctx: &UI) -> ProgressBarValue {
        let sys_value = unsafe { ui_sys::uiProgressBarValue(self.uiProgressBar) };
        if sys_value.is_negative() {
            assert!(
                sys_value == -1,
                "if ProgressBar value is negative it can only be -1"
            );
            ProgressBarValue::Indeterminate
        } else {
            ProgressBarValue::Determinate(sys_value as u32)
        }
    }
}
