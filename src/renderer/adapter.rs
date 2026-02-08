use i_slint_core::{renderer::Renderer, window::WindowAdapter};
use raw_window_handle::{
    HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};

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

// ---------- SbDisplayWindowHandle ---------- //

pub(super) struct SbDisplayWindowHandle {
    pub(super) display: RawDisplayHandle,
    pub(super) window: RawWindowHandle,
}

impl SbDisplayWindowHandle {
    pub(super) fn new<'a>(window: &baseview::Window<'a>) -> Self {
        Self {
            display: window.display_handle().expect("No display handle").as_raw(),
            window: window.window_handle().expect("No window handle").as_raw(),
        }
    }
}

impl HasDisplayHandle for SbDisplayWindowHandle {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, HandleError> {
        unsafe { Ok(raw_window_handle::DisplayHandle::borrow_raw(self.display)) }
    }
}

impl HasWindowHandle for SbDisplayWindowHandle {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, HandleError> {
        unsafe { Ok(raw_window_handle::WindowHandle::borrow_raw(self.window)) }
    }
}

unsafe impl Send for SbDisplayWindowHandle {}
unsafe impl Sync for SbDisplayWindowHandle {}
