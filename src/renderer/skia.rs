use i_slint_core::{renderer::Renderer, window::WindowAdapter};
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};
use std::sync::Arc;

use super::{SbDisplayWindowHandle, SbRendererAdapter};

// ---------- SbSkiaRendererAdapter ---------- //

pub(super) struct SbSkiaRendererAdapter {
    renderer: SkiaRenderer,
}

impl Default for SbSkiaRendererAdapter {
    fn default() -> Self {
        Self {
            renderer: SkiaRenderer::default(&SkiaSharedContext::default()),
        }
    }
}

impl SbRendererAdapter for SbSkiaRendererAdapter {
    fn set_window(
        &self,
        window: &baseview::Window,
        window_adapter: &dyn WindowAdapter,
    ) -> Result<(), String> {
        let handle = Arc::new(SbDisplayWindowHandle::new(window));
        self.renderer
            .set_window_handle(handle.clone(), handle, window_adapter.size(), None)
            .map_err(|err| format!("Skia set window error: {err}"))
    }

    fn render(&self, _window_adapter: &dyn WindowAdapter) -> Result<(), String> {
        self.renderer
            .render()
            .map_err(|err| format!("Skia render error: {err}"))
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}
