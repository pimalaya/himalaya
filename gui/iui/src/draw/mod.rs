//! Functions and types related to 2D vector graphics.

mod brush;
mod context;
mod path;
mod strokeparams;
mod transform;

pub use self::brush::*;
pub use self::context::*;
pub use self::path::*;
pub use self::strokeparams::*;
pub use self::transform::*;

pub use ui_sys::uiDrawDefaultMiterLimit as DEFAULT_MITER_LIMIT;

// pub struct FontFamilies {
//     ui_draw_font_families: *mut uiDrawFontFamilies,
// }

// impl Drop for FontFamilies {
//     fn drop(&mut self) {
//         unsafe { ui_sys::uiDrawFreeFontFamilies(self.ui_draw_font_families) }
//     }
// }

// impl FontFamilies {
//     pub fn list(_ctx: &UI) ->  FontFamilies {
//         unsafe {
//             FontFamilies {
//                 ui_draw_font_families: ui_sys::uiDrawListFontFamilies(),
//             }
//         }
//     }

//     pub fn len(&self, _ctx: &UI) -> u64 {
//         unsafe { ui_sys::uiDrawFontFamiliesNumFamilies(self.ui_draw_font_families) }
//     }

//     pub fn family(&self, ctx: &UI, index: u64) -> Text {
//         assert!(index < self.len(ctx));
//         unsafe {
//             Text::new(ui_sys::uiDrawFontFamiliesFamily(
//                 self.ui_draw_font_families,
//                 index,
//             ))
//         }
//     }
// }

// pub mod text {
//     use ui::UI;
//     // use ffi_utils;
//     use std::os::raw::c_char;
//     use std::ffi::{CStr, CString};
//     use std::mem;
//     use ui_sys::{self, uiDrawTextFont, uiDrawTextFontDescriptor, uiDrawTextLayout};

//     pub use ui_sys::uiDrawTextWeight as Weight;
//     pub use ui_sys::uiDrawTextItalic as Italic;
//     pub use ui_sys::uiDrawTextStretch as Stretch;
//     pub use ui_sys::uiDrawTextFontMetrics as FontMetrics;

//     pub struct FontDescriptor {
//         family: CString,
//         pub size: f64,
//         pub weight: Weight,
//         pub italic: Italic,
//         pub stretch: Stretch,
//     }

//     impl FontDescriptor {

//         pub fn new(
//             _ctx: &UI,
//             family: &str,
//             size: f64,
//             weight: Weight,
//             italic: Italic,
//             stretch: Stretch,
//         ) -> FontDescriptor {
//             FontDescriptor {
//                 family: CString::new(family.as_bytes().to_vec()).unwrap(),
//                 size: size,
//                 weight: weight,
//                 italic: italic,
//                 stretch: stretch,
//             }
//         }

//         /// FIXME(pcwalton): Should this return an Option?

//         pub fn load_closest_font(&self, _ctx: &UI) -> Font {
//             unsafe {
//                 let font_descriptor = uiDrawTextFontDescriptor {
//                     Family: self.family.as_ptr(),
//                     Size: self.size,
//                     Weight: self.weight,
//                     Italic: self.italic,
//                     Stretch: self.stretch,
//                 };
//                 Font {
//                     ui_draw_text_font: ui_sys::uiDrawLoadClosestFont(&font_descriptor),
//                 }
//             }
//         }

//         pub fn family(&self) -> &str {
//             self.family.to_str().unwrap()
//         }
//     }

//     pub struct Font {
//         ui_draw_text_font: *mut uiDrawTextFont,
//     }

//     impl Drop for Font {

//         fn drop(&mut self) {
//             unsafe { ui_sys::uiDrawFreeTextFont(self.ui_draw_text_font) }
//         }
//     }

//     impl Font {

//         pub unsafe fn from_ui_draw_text_font(ui_draw_text_font: *mut uiDrawTextFont) -> Font {
//             Font {
//                 ui_draw_text_font: ui_draw_text_font,
//             }
//         }

//         pub fn handle(&self, _ctx: &UI) -> usize {
//             unsafe { ui_sys::uiDrawTextFontHandle(self.ui_draw_text_font) }
//         }

//         pub fn describe(&self, _ctx: &UI) -> FontDescriptor {
//             unsafe {
//                 let mut ui_draw_text_font_descriptor = mem::uninitialized();
//                 ui_sys::uiDrawTextFontDescribe(
//                     self.ui_draw_text_font,
//                     &mut ui_draw_text_font_descriptor,
//                 );
//                 let family = CStr::from_ptr(ui_draw_text_font_descriptor.Family)
//                     .to_bytes()
//                     .to_vec();
//                 let font_descriptor = FontDescriptor {
//                     family: CString::new(family).unwrap(),
//                     size: ui_draw_text_font_descriptor.Size,
//                     weight: ui_draw_text_font_descriptor.Weight,
//                     italic: ui_draw_text_font_descriptor.Italic,
//                     stretch: ui_draw_text_font_descriptor.Stretch,
//                 };
//                 ui_sys::uiFreeText(ui_draw_text_font_descriptor.Family as *mut c_char);
//                 font_descriptor
//             }
//         }

//         pub fn metrics(&self, _ctx: &UI) -> FontMetrics {
//             unsafe {
//                 let mut metrics = mem::uninitialized();
//                 ui_sys::uiDrawTextFontGetMetrics(self.ui_draw_text_font, &mut metrics);
//                 metrics
//             }
//         }
//     }

//     pub struct Layout {
//         ui_draw_text_layout: *mut uiDrawTextLayout,
//     }

//     impl Drop for Layout {

//         fn drop(&mut self) {
//             unsafe { ui_sys::uiDrawFreeTextLayout(self.ui_draw_text_layout) }
//         }
//     }

//     impl Layout {

//         pub fn new(_ctx: &UI, text: &str, default_font: &Font, width: f64) -> Layout {
//             unsafe {
//                 let c_string = CString::new(text.as_bytes().to_vec()).unwrap();
//                 Layout {
//                     ui_draw_text_layout: ui_sys::uiDrawNewTextLayout(
//                         c_string.as_ptr(),
//                         default_font.ui_draw_text_font,
//                         width,
//                     ),
//                 }
//             }
//         }

//         pub fn as_ui_draw_text_layout(&self) -> *mut uiDrawTextLayout {
//             self.ui_draw_text_layout
//         }

//         pub fn set_width(&self, _ctx: &UI, width: f64) {
//             unsafe { ui_sys::uiDrawTextLayoutSetWidth(self.ui_draw_text_layout, width) }
//         }

//         pub fn extents(&self, _ctx: &UI) -> (f64, f64) {
//             unsafe {
//                 let mut extents = (0.0, 0.0);
//                 ui_sys::uiDrawTextLayoutExtents(
//                     self.ui_draw_text_layout,
//                     &mut extents.0,
//                     &mut extents.1,
//                 );
//                 extents
//             }
//         }

//         pub fn set_color(&self, _ctx: &UI, start_char: i64, end_char: i64, r: f64, g: f64, b: f64, a: f64) {
//             unsafe {
//                 ui_sys::uiDrawTextLayoutSetColor(
//                     self.ui_draw_text_layout,
//                     start_char,
//                     end_char,
//                     r,
//                     g,
//                     b,
//                     a,
//                 )
//             }
//         }
//     }
// }
