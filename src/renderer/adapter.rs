use i_slint_core::{renderer::Renderer, window::WindowAdapter};

// ---------- SbRendererType ---------- //

pub enum SbRendererType {
    #[cfg(feature = "femtovg")]
    FemtoVG,
    #[cfg(feature = "skia")]
    Skia,
    #[cfg(feature = "software")]
    Software,
}

impl SbRendererType {
    pub(crate) fn create_adapter(&self) -> Box<dyn SbRendererAdapter> {
        match self {
            #[cfg(feature = "femtovg")]
            SbRendererType::FemtoVG => {
                Box::new(super::femtovg::SbFemtoVGRendererAdapter::default())
            }
            #[cfg(feature = "skia")]
            SbRendererType::Skia => Box::new(super::skia::SbSkiaRendererAdapter::default()),
            #[cfg(feature = "software")]
            SbRendererType::Software => {
                Box::new(super::software::SbSoftwareRendererAdapter::default())
            }
        }
    }
}

// ---------- SbRendererAdapter ---------- //

pub(crate) trait SbRendererAdapter {
    fn set_window(
        &self,
        window: &baseview::Window,
        window_adapter: &dyn WindowAdapter,
    ) -> Result<(), String>;
    fn render(&self, window_adapter: &dyn WindowAdapter) -> Result<(), String>;
    fn renderer(&self) -> &dyn Renderer;
}
