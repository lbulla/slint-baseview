use i_slint_core::{renderer::Renderer, window::WindowAdapter};
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};
use raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;

use super::SbRendererAdapter;

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
        let handle = Arc::new(HandleWrapper(window.handle()));
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

// ---------- HandleWrapper ---------- //

pub(super) struct HandleWrapper(baseview::Handle);

impl HasDisplayHandle for HandleWrapper {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

impl HasWindowHandle for HandleWrapper {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

unsafe impl Send for HandleWrapper {}
unsafe impl Sync for HandleWrapper {}
