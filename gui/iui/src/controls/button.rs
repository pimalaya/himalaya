use super::Control;
use callback_helpers::{from_void_ptr, to_heap_ptr};
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::c_void;
use ui::UI;
use ui_sys::{self, uiButton, uiControl};

define_control! {
    /// A textual button which users can click on, causing a callback to run.
    rust_type: Button,
    sys_type: uiButton
}

impl Button {
    /// Create a new button with the given text as its label.
    pub fn new(_ctx: &UI, text: &str) -> Button {
        unsafe {
            let c_string = CString::new(text.as_bytes().to_vec()).unwrap();
            Button::from_raw(ui_sys::uiNewButton(c_string.as_ptr()))
        }
    }

    /// Get a copy of the existing text on the button.
    pub fn text(&self, _ctx: &UI) -> String {
        unsafe {
            CStr::from_ptr(ui_sys::uiButtonText(self.uiButton))
                .to_string_lossy()
                .into_owned()
        }
    }

    /// Get a reference to the existing text on the button.
    pub fn text_ref(&self, _ctx: &UI) -> &CStr {
        unsafe { CStr::from_ptr(ui_sys::uiButtonText(self.uiButton)) }
    }

    /// Set the text on the button.
    pub fn set_text(&mut self, _ctx: &UI, text: &str) {
        unsafe {
            let c_string = CString::new(text.as_bytes().to_vec()).unwrap();
            ui_sys::uiButtonSetText(self.uiButton, c_string.as_ptr())
        }
    }

    /// Run the given callback when the button is clicked.
    pub fn on_clicked<'ctx, F>(&mut self, _ctx: &'ctx UI, callback: F)
    where
        F: FnMut(&mut Button) + 'static,
    {
        extern "C" fn c_callback<G>(button: *mut uiButton, data: *mut c_void)
        where
            G: FnMut(&mut Button),
        {
            let mut button = Button { uiButton: button };
            unsafe {
                from_void_ptr::<G>(data)(&mut button);
            }
        }
        unsafe {
            ui_sys::uiButtonOnClicked(self.uiButton, Some(c_callback::<F>), to_heap_ptr(callback));
        }
    }
}
