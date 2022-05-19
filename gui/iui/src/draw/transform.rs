use std::mem;
use std::ops::Mul;
use ui_sys::{self, uiDrawMatrix};

/// A transformation which can be applied to the contents of a DrawContext.
#[derive(Copy, Clone, Debug)]
pub struct Transform {
    ui_matrix: uiDrawMatrix,
}

impl Transform {
    /// Create a Transform from an existing raw uiDrawMatrix.
    pub fn from_ui_matrix(ui_matrix: &uiDrawMatrix) -> Transform {
        Transform {
            ui_matrix: *ui_matrix,
        }
    }

    /// Create a new Transform that does nothing.
    pub fn identity() -> Transform {
        unsafe {
            let mut matrix = mem::MaybeUninit::uninit();
            ui_sys::uiDrawMatrixSetIdentity(matrix.as_mut_ptr());
            Transform::from_ui_matrix(&matrix.assume_init())
        }
    }

    /// Modify this Transform to translate by the given amounts.
    pub fn translate(&mut self, x: f64, y: f64) {
        unsafe { ui_sys::uiDrawMatrixTranslate(&mut self.ui_matrix, x, y) }
    }

    /// Modify this Transform to scale by the given amounts from the given center.
    pub fn scale(&mut self, x_center: f64, y_center: f64, x: f64, y: f64) {
        unsafe { ui_sys::uiDrawMatrixScale(&mut self.ui_matrix, x_center, y_center, x, y) }
    }

    /// Modify this Transform to rotate around the given center by the given angle.
    pub fn rotate(&mut self, x: f64, y: f64, angle: f64) {
        unsafe { ui_sys::uiDrawMatrixRotate(&mut self.ui_matrix, x, y, angle) }
    }

    /// Modify this Transform to skew from the given point by the given amount.
    pub fn skew(&mut self, x: f64, y: f64, xamount: f64, yamount: f64) {
        unsafe { ui_sys::uiDrawMatrixSkew(&mut self.ui_matrix, x, y, xamount, yamount) }
    }

    /// Compose this Transform with another, creating a Transform which represents both operations.
    pub fn compose(&mut self, src: &Transform) {
        unsafe { ui_sys::uiDrawMatrixMultiply(&mut self.ui_matrix, src.ptr()) }
    }

    /// Returns true if inverting this Transform is possible.
    pub fn invertible(&self) -> bool {
        unsafe {
            ui_sys::uiDrawMatrixInvertible(
                &self.ui_matrix as *const uiDrawMatrix as *mut uiDrawMatrix,
            ) != 0
        }
    }

    /// Attempts to invert the Transform, returning true if it succeeded and false if it failed.
    pub fn invert(&mut self) -> bool {
        unsafe { ui_sys::uiDrawMatrixInvert(&mut self.ui_matrix) != 0 }
    }

    pub fn transform_point(&self, mut point: (f64, f64)) -> (f64, f64) {
        unsafe {
            ui_sys::uiDrawMatrixTransformPoint(
                &self.ui_matrix as *const uiDrawMatrix as *mut uiDrawMatrix,
                &mut point.0,
                &mut point.1,
            );
            point
        }
    }

    pub fn transform_size(&self, mut size: (f64, f64)) -> (f64, f64) {
        unsafe {
            ui_sys::uiDrawMatrixTransformSize(
                &self.ui_matrix as *const uiDrawMatrix as *mut uiDrawMatrix,
                &mut size.0,
                &mut size.1,
            );
            size
        }
    }

    pub fn ptr(&self) -> *mut uiDrawMatrix {
        &self.ui_matrix as *const uiDrawMatrix as *mut uiDrawMatrix
    }
}

impl Mul<Transform> for Transform {
    type Output = Transform;

    fn mul(mut self, other: Transform) -> Transform {
        self.compose(&other);
        self
    }
}
