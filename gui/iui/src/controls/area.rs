//! Provides a way to allocate an area in the window for custom drawing.

use controls::Control;
use draw;
use std::mem;
use std::os::raw::c_int;
use ui::UI;
pub use ui_sys::uiExtKey as ExtKey;
use ui_sys::{
    self, uiArea, uiAreaDrawParams, uiAreaHandler, uiAreaKeyEvent, uiAreaMouseEvent, uiControl,
};

pub trait AreaHandler {
    fn draw(&mut self, _area: &Area, _area_draw_params: &AreaDrawParams) {}
    fn mouse_event(&mut self, _area: &Area, _area_mouse_event: &AreaMouseEvent) {}
    fn mouse_crossed(&mut self, _area: &Area, _left: bool) {}
    fn drag_broken(&mut self, _area: &Area) {}
    fn key_event(&mut self, _area: &Area, _area_key_event: &AreaKeyEvent) -> bool {
        true
    }
}

#[repr(C)]
struct RustAreaHandler {
    ui_area_handler: uiAreaHandler,
    trait_object: Box<dyn AreaHandler>,
}

impl RustAreaHandler {
    fn new(_ctx: &UI, trait_object: Box<dyn AreaHandler>) -> Box<RustAreaHandler> {
        return Box::new(RustAreaHandler {
            ui_area_handler: uiAreaHandler {
                Draw: Some(draw),
                MouseEvent: Some(mouse_event),
                MouseCrossed: Some(mouse_crossed),
                DragBroken: Some(drag_broken),
                KeyEvent: Some(key_event),
            },
            trait_object,
        });

        extern "C" fn draw(
            ui_area_handler: *mut uiAreaHandler,
            ui_area: *mut uiArea,
            ui_area_draw_params: *mut uiAreaDrawParams,
        ) {
            unsafe {
                let area = Area::from_ui_area(ui_area);
                let area_draw_params =
                    AreaDrawParams::from_ui_area_draw_params(&*ui_area_draw_params);
                (*(ui_area_handler as *mut RustAreaHandler))
                    .trait_object
                    .draw(&area, &area_draw_params);
                mem::forget(area_draw_params);
                mem::forget(area);
            }
        }

        extern "C" fn mouse_event(
            ui_area_handler: *mut uiAreaHandler,
            ui_area: *mut uiArea,
            ui_area_mouse_event: *mut uiAreaMouseEvent,
        ) {
            unsafe {
                let area = Area::from_ui_area(ui_area);
                let area_mouse_event =
                    AreaMouseEvent::from_ui_area_mouse_event(&*ui_area_mouse_event);
                (*(ui_area_handler as *mut RustAreaHandler))
                    .trait_object
                    .mouse_event(&area, &area_mouse_event);
                mem::forget(area_mouse_event);
                mem::forget(area);
            }
        }

        extern "C" fn mouse_crossed(
            ui_area_handler: *mut uiAreaHandler,
            ui_area: *mut uiArea,
            left: c_int,
        ) {
            unsafe {
                let area = Area::from_ui_area(ui_area);
                (*(ui_area_handler as *mut RustAreaHandler))
                    .trait_object
                    .mouse_crossed(&area, left != 0);
                mem::forget(area);
            }
        }

        extern "C" fn drag_broken(ui_area_handler: *mut uiAreaHandler, ui_area: *mut uiArea) {
            unsafe {
                let area = Area::from_ui_area(ui_area);
                (*(ui_area_handler as *mut RustAreaHandler))
                    .trait_object
                    .drag_broken(&area);
                mem::forget(area);
            }
        }

        extern "C" fn key_event(
            ui_area_handler: *mut uiAreaHandler,
            ui_area: *mut uiArea,
            ui_area_key_event: *mut uiAreaKeyEvent,
        ) -> c_int {
            unsafe {
                let area = Area::from_ui_area(ui_area);
                let area_key_event = AreaKeyEvent::from_ui_area_key_event(&*ui_area_key_event);
                let result = (*(ui_area_handler as *mut RustAreaHandler))
                    .trait_object
                    .key_event(&area, &area_key_event);
                mem::forget(area_key_event);
                mem::forget(area);
                result as c_int
            }
        }
    }
}

define_control! {
    /// A space on which the application can draw custom content.
    /// Area is a Control that represents a blank canvas that a program can draw on as
    /// it wishes. Areas also receive keyboard and mouse events, and programs can react
    /// to those as they see fit. Drawing and event handling are handled through an
    /// instance of a type that implements `AreaHandler` that every `Area` has; see
    /// `AreaHandler` for details.
    ///
    /// There are two types of areas. Non-scrolling areas are rectangular and have no
    /// scrollbars. Programs can draw on and get mouse events from any point in the
    /// `Area`, and the size of the Area is decided by package ui itself, according to
    /// the layout of controls in the Window the Area is located in and the size of said
    /// Window. There is no way to query the Area's size or be notified when its size
    /// changes; instead, you are given the area size as part of the draw and mouse event
    /// handlers, for use solely within those handlers.
    ///
    /// Scrolling areas have horziontal and vertical scrollbars. The amount that can be
    /// scrolled is determined by the area's size, which is decided by the programmer
    /// (both when creating the Area and by a call to SetSize). Only a portion of the
    /// Area is visible at any time; drawing and mouse events are automatically adjusted
    /// to match what portion is visible, so you do not have to worry about scrolling in
    /// your event handlers. AreaHandler has more information.
    ///
    /// The internal coordinate system of an Area is points, which are floating-point and
    /// device-independent. For more details, see `AreaHandler`. The size of a scrolling
    /// Area must be an exact integer number of points
    rust_type: Area,
    sys_type: uiArea
}

impl Area {
    /// Creates a new non-scrolling area.
    pub fn new(ctx: &UI, area_handler: Box<dyn AreaHandler>) -> Area {
        unsafe {
            let mut rust_area_handler = RustAreaHandler::new(ctx, area_handler);
            let area = Area::from_raw(ui_sys::uiNewArea(
                &mut *rust_area_handler as *mut RustAreaHandler as *mut uiAreaHandler,
            ));
            mem::forget(rust_area_handler);
            area
        }
    }

    /// Creates a new scrolling area.
    pub fn new_scrolling(
        ctx: &UI,
        area_handler: Box<dyn AreaHandler>,
        width: i64,
        height: i64,
    ) -> Area {
        unsafe {
            let mut rust_area_handler = RustAreaHandler::new(ctx, area_handler);
            let area = Area::from_raw(ui_sys::uiNewScrollingArea(
                &mut *rust_area_handler as *mut RustAreaHandler as *mut uiAreaHandler,
                width as i32,
                height as i32,
            ));
            mem::forget(rust_area_handler);
            area
        }
    }

    pub unsafe fn from_ui_area(ui_area: *mut uiArea) -> Area {
        Area { uiArea: ui_area }
    }

    /// Sets the size of the area in points.
    ///
    /// # Unsafety
    /// If called on a non-scrolling `Area`, this function's behavior is undefined.
    pub unsafe fn set_size(&self, _ctx: &UI, width: u64, height: u64) {
        // TODO: Check if the area is scrolling?
        ui_sys::uiAreaSetSize(self.uiArea, width as i32, height as i32);
    }

    /// Queues the entire `Area` to be redrawn. This function returns immediately;
    /// the `Area` is redrawn when the UI thread is next non-busy.
    pub fn queue_redraw_all(&self, _ctx: &UI) {
        unsafe { ui_sys::uiAreaQueueRedrawAll(self.uiArea) }
    }

    /// Scrolls the Area to show the given rectangle. This behavior is somewhat
    /// implementation defined, but you can assume that as much of the given rectangle
    /// as possible will be visible after this call.
    ///
    /// # Unsafety
    /// If called on a non-scrolling `Area`, this function's behavior is undefined.
    pub unsafe fn scroll_to(&self, _ctx: &UI, x: f64, y: f64, width: f64, height: f64) {
        // TODO: Make some way to check whether the given area is scrolling or not.
        ui_sys::uiAreaScrollTo(self.uiArea, x, y, width, height);
    }
}

/// Provides a drawing context that can be used to draw on an Area, and tells you
/// where to draw. See `AreaHandler` for introductory information.
///
/// Height and width values can change at any time, without generating an event,
/// so do not save them elsewhere.
///
/// The clipping rectangle parameters specify the only area in which drawing is allowed.
/// The system will ensure nothing is drawn outside that area, but drawing is far faster
/// if the program does not attempt to put things out of bounds.
pub struct AreaDrawParams {
    /// The `DrawContext` on which to draw. See `DrawContext` for how to draw.
    pub context: draw::DrawContext,

    /// The width of the `Area`, for non-scrolling `Area`s.
    pub area_width: f64,
    /// The height of the `Area`, for non-scrolling `Area`s.
    pub area_height: f64,

    /// Leftmost position of the clipping rectangle.
    pub clip_x: f64,
    /// Topmost position of the clipping rectangle.
    pub clip_y: f64,
    /// Width of the clipping rectangle.
    pub clip_width: f64,
    /// Height of the clipping rectangle.
    pub clip_height: f64,
}

impl AreaDrawParams {
    // TODO: check if UI is initialized?
    unsafe fn from_ui_area_draw_params(ui_area_draw_params: &uiAreaDrawParams) -> AreaDrawParams {
        AreaDrawParams {
            context: draw::DrawContext::from_ui_draw_context(ui_area_draw_params.Context),
            area_width: ui_area_draw_params.AreaWidth,
            area_height: ui_area_draw_params.AreaHeight,
            clip_x: ui_area_draw_params.ClipX,
            clip_y: ui_area_draw_params.ClipY,
            clip_width: ui_area_draw_params.ClipWidth,
            clip_height: ui_area_draw_params.ClipHeight,
        }
    }
}

bitflags! {
    pub struct Modifiers: u8 {
        const MODIFIER_CTRL = 1 << 0;
        const MODIFIER_ALT = 1 << 1;
        const MODIFIER_SHIFT = 1 << 2;
        const MODIFIER_SUPER = 1 << 3;
    }
}

#[derive(Copy, Clone, Debug)]
/// Represents a mouse event in an `Area`.
pub struct AreaMouseEvent {
    pub x: f64,
    pub y: f64,

    pub area_width: f64,
    pub area_height: f64,

    pub down: i32,
    pub up: i32,

    pub count: i32,

    pub modifiers: Modifiers,

    pub held_1_to_64: u64,
}

impl AreaMouseEvent {
    pub fn from_ui_area_mouse_event(ui_area_mouse_event: &uiAreaMouseEvent) -> AreaMouseEvent {
        AreaMouseEvent {
            x: ui_area_mouse_event.X,
            y: ui_area_mouse_event.Y,
            area_width: ui_area_mouse_event.AreaWidth,
            area_height: ui_area_mouse_event.AreaHeight,
            down: ui_area_mouse_event.Down,
            up: ui_area_mouse_event.Up,
            count: ui_area_mouse_event.Count,
            modifiers: Modifiers::from_bits(ui_area_mouse_event.Modifiers as u8)
                .unwrap_or(Modifiers::empty()),
            held_1_to_64: ui_area_mouse_event.Held1To64,
        }
    }
}

#[derive(Copy, Clone, Debug)]
/// A keypress or key release event for an `Area`.
pub struct AreaKeyEvent {
    pub key: u8,
    pub ext_key: ExtKey,
    pub modifier: Modifiers,
    pub modifiers: Modifiers,
    pub up: bool,
}

impl AreaKeyEvent {
    pub fn from_ui_area_key_event(ui_area_key_event: &uiAreaKeyEvent) -> AreaKeyEvent {
        AreaKeyEvent {
            key: ui_area_key_event.Key as u8,
            ext_key: ui_area_key_event.ExtKey,
            modifier: Modifiers::from_bits(ui_area_key_event.Modifier as u8)
                .unwrap_or(Modifiers::empty()),
            modifiers: Modifiers::from_bits(ui_area_key_event.Modifiers as u8)
                .unwrap_or(Modifiers::empty()),
            up: ui_area_key_event.Up != 0,
        }
    }
}
