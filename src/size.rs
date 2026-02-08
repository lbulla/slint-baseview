use serde::{Deserialize, Serialize};

// ---------- SbLogicalSize ---------- //

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct SbLogicalSize {
    pub width: f64,
    pub height: f64,
}

impl SbLogicalSize {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn from_physical(physical: &SbPhysicalSize, scale_factor: f64) -> Self {
        Self::new(
            physical.width as f64 / scale_factor,
            physical.height as f64 / scale_factor,
        )
    }

    pub fn to_physical(&self, scale_factor: f64) -> SbPhysicalSize {
        SbPhysicalSize::from_logical(self, scale_factor)
    }
}

// ---------- SbPhysicalSize ---------- //

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct SbPhysicalSize {
    pub width: u32,
    pub height: u32,
}

impl SbPhysicalSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn from_logical(logical: &SbLogicalSize, scale_factor: f64) -> Self {
        Self::new(
            (logical.width * scale_factor).round() as _,
            (logical.height * scale_factor).round() as _,
        )
    }

    pub fn to_logical(&self, scale_factor: f64) -> SbLogicalSize {
        SbLogicalSize::from_physical(self, scale_factor)
    }
}
