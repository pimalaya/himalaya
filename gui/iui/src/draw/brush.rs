use draw::DrawContext;
use std::marker::PhantomData;
use std::ptr;
use ui_sys::{self, uiDrawBrush};

pub use ui_sys::uiDrawBrushGradientStop as BrushGradientStop;

/// Used to determine how a given stroke or fill is drawn.
#[derive(Clone, Debug)]
pub enum Brush {
    Solid(SolidBrush),
    LinearGradient(LinearGradientBrush),
    RadialGradient(RadialGradientBrush),
    Image,
}

/// A reference to a DrawBrush
#[derive(Clone, Debug)]
pub struct BrushRef<'a> {
    ui_draw_brush: uiDrawBrush,
    phantom: PhantomData<&'a uiDrawBrush>,
}

impl Brush {
    pub fn as_ui_draw_brush_ref(&self, _ctx: &DrawContext) -> BrushRef {
        match *self {
            Brush::Solid(ref solid_brush) => BrushRef {
                ui_draw_brush: uiDrawBrush {
                    Type: ui_sys::uiDrawBrushTypeSolid as u32,

                    R: solid_brush.r,
                    G: solid_brush.g,
                    B: solid_brush.b,
                    A: solid_brush.a,

                    X0: 0.0,
                    Y0: 0.0,
                    X1: 0.0,
                    Y1: 0.0,
                    OuterRadius: 0.0,
                    Stops: ptr::null_mut(),
                    NumStops: 0,
                },
                phantom: PhantomData,
            },
            Brush::LinearGradient(ref linear_gradient_brush) => BrushRef {
                ui_draw_brush: uiDrawBrush {
                    Type: ui_sys::uiDrawBrushTypeLinearGradient as u32,

                    R: 0.0,
                    G: 0.0,
                    B: 0.0,
                    A: 0.0,

                    X0: linear_gradient_brush.start_x,
                    Y0: linear_gradient_brush.start_y,
                    X1: linear_gradient_brush.end_x,
                    Y1: linear_gradient_brush.end_y,
                    OuterRadius: 0.0,
                    Stops: linear_gradient_brush.stops.as_ptr() as *mut BrushGradientStop,
                    NumStops: linear_gradient_brush.stops.len() as u64,
                },
                phantom: PhantomData,
            },
            Brush::RadialGradient(ref radial_gradient_brush) => BrushRef {
                ui_draw_brush: uiDrawBrush {
                    Type: ui_sys::uiDrawBrushTypeRadialGradient as u32,

                    R: 0.0,
                    G: 0.0,
                    B: 0.0,
                    A: 0.0,

                    X0: radial_gradient_brush.start_x,
                    Y0: radial_gradient_brush.start_y,
                    X1: radial_gradient_brush.outer_circle_center_x,
                    Y1: radial_gradient_brush.outer_circle_center_y,
                    OuterRadius: radial_gradient_brush.outer_radius,
                    Stops: radial_gradient_brush.stops.as_ptr() as *mut BrushGradientStop,
                    NumStops: radial_gradient_brush.stops.len() as u64,
                },
                phantom: PhantomData,
            },
            Brush::Image => {
                // These don't work yet in `libui`, but just for completeness' sake…
                BrushRef {
                    ui_draw_brush: uiDrawBrush {
                        Type: ui_sys::uiDrawBrushTypeImage as u32,

                        R: 0.0,
                        G: 0.0,
                        B: 0.0,
                        A: 0.0,

                        X0: 0.0,
                        Y0: 0.0,
                        X1: 0.0,
                        Y1: 0.0,
                        OuterRadius: 0.0,
                        Stops: ptr::null_mut(),
                        NumStops: 0,
                    },
                    phantom: PhantomData,
                }
            }
        }
    }
}

impl<'a> BrushRef<'a> {
    /// Return the underlying uiDrawBrush for this BrushRef as a mutable pointer.
    pub unsafe fn ptr(&'a self) -> *mut uiDrawBrush {
        &self.ui_draw_brush as *const uiDrawBrush as *mut uiDrawBrush
    }
}

/// A brush that paints all pixels with the same color, respecting alpha.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SolidBrush {
    /// Red component of the color
    pub r: f64,
    /// Green component of the color
    pub g: f64,
    /// Blue component of the color
    pub b: f64,
    /// Alpha (α) component of the color (that is, opacity).
    pub a: f64,
}

/// A brush that paints a linear gradient.
#[derive(Clone, Debug)]
pub struct LinearGradientBrush {
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
    pub stops: Vec<BrushGradientStop>,
}

/// A brush that paints a radial gradient.
#[derive(Clone, Debug)]
pub struct RadialGradientBrush {
    pub start_x: f64,
    pub start_y: f64,
    pub outer_circle_center_x: f64,
    pub outer_circle_center_y: f64,
    pub outer_radius: f64,
    pub stops: Vec<BrushGradientStop>,
}
