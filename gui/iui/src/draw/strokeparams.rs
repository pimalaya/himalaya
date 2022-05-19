use draw::DrawContext;
use std::marker::PhantomData;
use std::os::raw::c_double;
use ui_sys::uiDrawStrokeParams;

pub use ui_sys::uiDrawLineCap as LineCap;
pub use ui_sys::uiDrawLineJoin as LineJoin;

#[derive(Clone, Debug)]
pub struct StrokeParams {
    pub cap: LineCap,
    pub join: LineJoin,
    pub thickness: f64,
    pub miter_limit: f64,
    pub dashes: Vec<f64>,
    pub dash_phase: f64,
}

#[derive(Clone, Debug)]
pub struct StrokeParamsRef<'a> {
    ui_draw_stroke_params: uiDrawStrokeParams,
    phantom: PhantomData<&'a uiDrawStrokeParams>,
}

impl StrokeParams {
    pub fn as_stroke_params_ref(&self, _ctx: &DrawContext) -> StrokeParamsRef {
        StrokeParamsRef {
            ui_draw_stroke_params: uiDrawStrokeParams {
                Cap: self.cap,
                Join: self.join,
                Thickness: self.thickness,
                MiterLimit: self.miter_limit,
                Dashes: self.dashes.as_ptr() as *mut c_double,
                NumDashes: self.dashes.len() as u64,
                DashPhase: self.dash_phase,
            },
            phantom: PhantomData,
        }
    }
}

impl<'a> StrokeParamsRef<'a> {
    /// Returns the underlying uiDrawStrokeParams.
    pub unsafe fn ptr(&self) -> *mut uiDrawStrokeParams {
        &self.ui_draw_stroke_params as *const uiDrawStrokeParams as *mut uiDrawStrokeParams
    }
}
