/// Defines a new control, creating a Rust wrapper, a `Deref` implementation, and a destructor.
/// An example of use:
/// ```ignore
///     define_control!{
///         /// Some documentation
///         #[attribute(whatever="something")]
///         rust_type: Slider,
///         sys_type: uiSlider,
///      }
/// ```
macro_rules! define_control {
    // Match first any attributes (incl. doc comments) and then the actual invocation
    {$(#[$attr:meta])* rust_type: $rust_type:ident, sys_type: $sys_type:ident$(,)* } => {
        #[allow(non_snake_case)]
        // Include all attributes
        $(#[$attr])*
        pub struct $rust_type {
            $sys_type: *mut $sys_type,
        }

        impl Drop for $rust_type {
            fn drop(&mut self) {
                // For now this does nothing, but in the future, when `libui` supports proper
                // memory management, this will likely need to twiddle reference counts.
            }
        }

        impl Clone for $rust_type {
            fn clone(&self) -> $rust_type {
                $rust_type {
                    $sys_type: self.$sys_type,
                }
            }
        }

        impl Into<Control> for $rust_type {
            fn into(self) -> Control {
                unsafe {
                    let control = Control::from_ui_control(self.$sys_type as *mut uiControl);
                    mem::forget(self);
                    control
                }
            }
        }

        impl $rust_type {
            // Show this control to the user. This will also show its non-hidden children.
            pub fn show(&mut self, _ctx: &UI) {
                let control: Control = self.clone().into();
                unsafe { ui_sys::uiControlShow(control.ui_control) }
            }

            // Hide this control from the user. This will hide its children.
            pub fn hide(&mut self, _ctx: &UI) {
                let control: Control = self.clone().into();
                unsafe { ui_sys::uiControlHide(control.ui_control) }
            }

            // Enable this control.
            pub fn enable(&mut self, _ctx: &UI) {
                let control: Control = self.clone().into();
                unsafe { ui_sys::uiControlEnable(control.ui_control) }
            }

            // Disable this control.
            pub fn disable(&mut self, _ctx: &UI) {
                let control: Control = self.clone().into();
                unsafe { ui_sys::uiControlDisable(control.ui_control) }
            }

            /// Create an `iui` struct for this control from the raw pointer for it.
            ///
            /// # Unsafety
            /// The given pointer must point to a valid control or memory unsafety may result.
            #[allow(non_snake_case)]
            #[allow(unused)]
            pub unsafe fn from_raw($sys_type: *mut $sys_type) -> $rust_type {
                $rust_type {
                    $sys_type: $sys_type
                }
            }

            /// Return the underlying pointer for this control.
            #[allow(non_snake_case)]
            pub fn ptr(&self) -> *mut $sys_type {
                self.$sys_type
            }
        }
    }
}
