use glutin::{
    config::ConfigTemplate,
    context::{ContextAttributesBuilder, PossiblyCurrentContext, PossiblyCurrentGlContext},
    display::{Display, DisplayApiPreference, GetGlDisplay},
    prelude::{GlDisplay, NotCurrentGlContext},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface},
};
use i_slint_core::{renderer::Renderer, window::WindowAdapter};
use i_slint_renderer_femtovg::{
    FemtoVGOpenGLRenderer, FemtoVGOpenGLRendererExt, FemtoVGRendererExt, opengl::OpenGLInterface,
};
use std::{
    error::Error,
    ffi::{CStr, c_void},
    num::NonZeroU32,
};

use super::{SbDisplayWindowHandle, SbRendererAdapter};

// ---------- SbFemtoVGRendererAdapter ---------- //

pub(super) struct SbFemtoVGRendererAdapter {
    renderer: FemtoVGOpenGLRenderer,
}

impl Default for SbFemtoVGRendererAdapter {
    fn default() -> Self {
        Self {
            renderer: FemtoVGOpenGLRenderer::new_suspended(),
        }
    }
}

impl SbRendererAdapter for SbFemtoVGRendererAdapter {
    fn set_window(
        &self,
        window: &baseview::Window,
        window_adapter: &dyn WindowAdapter,
    ) -> Result<(), String> {
        let handle = SbDisplayWindowHandle::new(window);

        cfg_if::cfg_if! {
            if #[cfg(target_os = "macos")] {
                let display_api_preference = DisplayApiPreference::Cgl;
            } else if #[cfg(not(target_family = "windows"))] {
                let display_api_preference = DisplayApiPreference::Egl;
            } else {
                let display_api_preference = DisplayApiPreference::EglThenWgl(Some(handle.window));
            }
        }
        let display = unsafe { Display::new(handle.display, display_api_preference) }
            .map_err(|err| format!("FemtoVG display error: {err}"))?;

        let config = unsafe { display.find_configs(ConfigTemplate::default()) }
            .map_err(|err| format!("FemtoVG configs error: {err}"))?
            .next();
        let Some(config) = config else {
            return Err("FemtoVG no config".into());
        };

        let context_attributes = ContextAttributesBuilder::new().build(Some(handle.window));
        let context = unsafe { display.create_context(&config, &context_attributes) }
            .map_err(|err| format!("FemtoVG context error: {err}"))?;

        let size = window_adapter.size();
        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            handle.window,
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );
        let surface = unsafe { display.create_window_surface(&config, &surface_attributes) }
            .map_err(|err| format!("FemtoVG surface error: {err}"))?;

        self.renderer
            .set_opengl_context(SbOpenGLInterface {
                context: context
                    .make_current(&surface)
                    .map_err(|err| format!("FemtoVG current context error: {err}"))?,
                surface,
            })
            .map_err(|err| format!("FemtoVG renderer error: {err}"))?;

        Ok(())
    }

    fn render(&self, _window_adapter: &dyn WindowAdapter) -> Result<(), String> {
        self.renderer
            .render()
            .map_err(|err| format!("FemtoVG render error: {err}"))
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}

// ---------- SbOpenGLInterface ---------- //

struct SbOpenGLInterface {
    context: PossiblyCurrentContext,
    surface: Surface<WindowSurface>,
}

unsafe impl OpenGLInterface for SbOpenGLInterface {
    fn ensure_current(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.context
            .make_current(&self.surface)
            .map_err(|err| format!("FemtoVG ensure current error: {err}").into())
    }

    fn swap_buffers(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.surface
            .swap_buffers(&self.context)
            .map_err(|err| format!("FemotVG swap buffers error: {err}").into())
    }

    fn resize(
        &self,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.surface.resize(&self.context, width, height);
        Ok(())
    }

    fn get_proc_address(&self, name: &CStr) -> *const c_void {
        self.context.display().get_proc_address(name)
    }
}
