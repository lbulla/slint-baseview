use i_slint_core::{api::Window, renderer::Renderer};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use std::sync::Arc;

#[cfg(feature = "femtovg")]
use glutin::{
    config::ConfigTemplate,
    context::{ContextAttributesBuilder, PossiblyCurrentContext, PossiblyCurrentGlContext},
    display::{Display, DisplayApiPreference, GetGlDisplay},
    prelude::{GlDisplay, NotCurrentGlContext},
    surface::{GlSurface, SurfaceAttributesBuilder, WindowSurface},
};
#[cfg(feature = "femtovg")]
use i_slint_renderer_femtovg::{
    FemtoVGOpenGLRenderer, FemtoVGOpenGLRendererExt, FemtoVGRendererExt, opengl::OpenGLInterface,
};

#[cfg(feature = "skia")]
use i_slint_renderer_skia::{SkiaRenderer, SkiaSharedContext};

#[cfg(feature = "software")]
use bytemuck::{AnyBitPattern, NoUninit, Zeroable, cast_slice_mut};
#[cfg(feature = "software")]
use i_slint_renderer_software::{PremultipliedRgbaColor, SoftwareRenderer, TargetPixel};
#[cfg(feature = "software")]
use softbuffer::Context;
#[cfg(feature = "software")]
use std::{cell::RefCell, ops::DerefMut};

// ---------- EmbeddedRendererAdapter ---------- //

pub(crate) trait EmbeddedRendererAdapter {
    fn set_window(
        &self,
        baseview_window: &baseview::Window,
        slint_window: &Window,
    ) -> Result<(), String>;
    fn render(&self, slint_window: &Window) -> Result<(), String>;
    fn renderer(&self) -> &dyn Renderer;
}

// ---------- FemtoVG ---------- //

#[cfg(feature = "femtovg")]
pub(crate) struct EmbeddedFemtoVGRendererAdapter {
    renderer: FemtoVGOpenGLRenderer,
}

#[cfg(feature = "femtovg")]
impl Default for EmbeddedFemtoVGRendererAdapter {
    fn default() -> Self {
        Self {
            renderer: FemtoVGOpenGLRenderer::new_suspended(),
        }
    }
}

#[cfg(feature = "femtovg")]
impl EmbeddedRendererAdapter for EmbeddedFemtoVGRendererAdapter {
    fn set_window(
        &self,
        baseview_window: &baseview::Window,
        slint_window: &Window,
    ) -> Result<(), String> {
        let raw_window_handle = baseview_window
            .window_handle()
            .expect("No window handle")
            .as_raw();
        let raw_display_handle = baseview_window
            .display_handle()
            .expect("No display handle")
            .as_raw();

        cfg_if::cfg_if! {
            if #[cfg(target_os = "macos")] {
                let display_api_preference = DisplayApiPreference::Cgl;
            } else if #[cfg(not(target_family = "windows"))] {
                let display_api_preference = DisplayApiPreference::Egl;
            } else {
                let display_api_preference = DisplayApiPreference::EglThenWgl(Some(raw_window_handle));
            }
        }
        let display = unsafe { Display::new(raw_display_handle, display_api_preference) }
            .map_err(|err| format!("FemtoVG display error: {err}"))?;

        let config = unsafe { display.find_configs(ConfigTemplate::default()) }
            .map_err(|err| format!("FemtoVG configs error: {err}"))?
            .next();
        let Some(config) = config else {
            return Err("FemtoVG no config".into());
        };

        let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));
        let context = unsafe { display.create_context(&config, &context_attributes) }
            .map_err(|err| format!("FemtoVG context error: {err}"))?;

        let size = slint_window.size();
        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            std::num::NonZeroU32::new(size.width).unwrap(),
            std::num::NonZeroU32::new(size.height).unwrap(),
        );
        let surface = unsafe { display.create_window_surface(&config, &surface_attributes) }
            .map_err(|err| format!("FemtoVG surface error: {err}"))?;

        self.renderer
            .set_opengl_context(FemtoVGOpenGLInterface {
                context: context
                    .make_current(&surface)
                    .map_err(|err| format!("FemtoVG current context error: {err}"))?,
                surface,
            })
            .map_err(|err| format!("FemtoVG renderer error: {err}"))?;

        Ok(())
    }

    fn render(&self, _slint_window: &Window) -> Result<(), String> {
        self.renderer
            .render()
            .map_err(|err| format!("FemtoVG render error: {err}"))
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}

#[cfg(feature = "femtovg")]
struct FemtoVGOpenGLInterface {
    context: PossiblyCurrentContext,
    surface: glutin::surface::Surface<WindowSurface>,
}

#[cfg(feature = "femtovg")]
unsafe impl OpenGLInterface for FemtoVGOpenGLInterface {
    fn ensure_current(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.context
            .make_current(&self.surface)
            .map_err(|err| format!("FemtoVG ensure current error: {err}").into())
    }

    fn swap_buffers(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.surface
            .swap_buffers(&self.context)
            .map_err(|err| format!("FemotVG swap buffers error: {err}").into())
    }

    fn resize(
        &self,
        width: std::num::NonZeroU32,
        height: std::num::NonZeroU32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.surface.resize(&self.context, width, height);
        Ok(())
    }

    fn get_proc_address(&self, name: &std::ffi::CStr) -> *const std::ffi::c_void {
        self.context.display().get_proc_address(name)
    }
}

// ---------- Skia ---------- //

#[cfg(feature = "skia")]
pub(crate) struct EmbeddedSkiaRendererAdapter {
    renderer: SkiaRenderer,
}

#[cfg(feature = "skia")]
impl Default for EmbeddedSkiaRendererAdapter {
    fn default() -> Self {
        Self {
            renderer: SkiaRenderer::default(&SkiaSharedContext::default()),
        }
    }
}

#[cfg(feature = "skia")]
impl EmbeddedRendererAdapter for EmbeddedSkiaRendererAdapter {
    fn set_window(
        &self,
        baseview_window: &baseview::Window,
        slint_window: &Window,
    ) -> Result<(), String> {
        let window_wrapper = Arc::new(BaseviewWindowWrapper::new(baseview_window));
        self.renderer
            .set_window_handle(
                window_wrapper.clone(),
                window_wrapper,
                slint_window.size(),
                None,
            )
            .map_err(|err| format!("Skia set window error: {err}"))
    }

    fn render(&self, _slint_window: &Window) -> Result<(), String> {
        self.renderer
            .render()
            .map_err(|err| format!("Skia render error: {err}"))
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}

// ---------- Software ---------- //

#[cfg(feature = "software")]
#[derive(Default)]
pub(crate) struct EmbeddedSoftwareRendererAdapter {
    renderer: SoftwareRenderer,
    context: RefCell<Option<Context<Arc<BaseviewWindowWrapper>>>>,
    surface: RefCell<
        Option<softbuffer::Surface<Arc<BaseviewWindowWrapper>, Arc<BaseviewWindowWrapper>>>,
    >,
}

#[cfg(feature = "software")]
impl EmbeddedRendererAdapter for EmbeddedSoftwareRendererAdapter {
    fn set_window(
        &self,
        baseview_window: &baseview::Window,
        _slint_window: &Window,
    ) -> Result<(), String> {
        let window_wrapper = Arc::new(BaseviewWindowWrapper::new(baseview_window));
        let context = Context::new(window_wrapper.clone())
            .map_err(|err| format!("Software context error: {err}"))?;
        let surface = softbuffer::Surface::new(&context, window_wrapper)
            .map_err(|err| format!("Software surface error: {err}"))?;
        self.context.borrow_mut().replace(context);
        self.surface.borrow_mut().replace(surface);
        Ok(())
    }

    fn render(&self, slint_window: &Window) -> Result<(), String> {
        let mut surface = self.surface.borrow_mut();
        let Some(surface) = surface.as_mut() else {
            return Ok(());
        };

        let size = slint_window.size();
        surface
            .resize(
                std::num::NonZeroU32::new(size.width).unwrap(),
                std::num::NonZeroU32::new(size.height).unwrap(),
            )
            .map_err(|err| format!("Software resize error: {err}"))?;

        let mut buffer = surface
            .buffer_mut()
            .map_err(|err| format!("Software buffer error: {err}"))?;

        let soft_buffer: &mut [SoftBufferPixel] = cast_slice_mut(buffer.deref_mut());
        self.renderer.render(soft_buffer, size.width as _);
        buffer
            .present()
            .map_err(|err| format!("Software present error: {err}"))
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}

#[cfg(feature = "software")]
#[derive(Clone, Copy, Zeroable)]
struct SoftBufferPixel(u32);

#[cfg(feature = "software")]
impl TargetPixel for SoftBufferPixel {
    fn blend(&mut self, color: PremultipliedRgbaColor) {
        let mut x = PremultipliedRgbaColor::from(*self);
        x.blend(color);
        *self = x.into();
    }

    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self(0xff000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    fn background() -> Self {
        Self(0)
    }
}

#[cfg(feature = "software")]
impl From<PremultipliedRgbaColor> for SoftBufferPixel {
    fn from(pixel: PremultipliedRgbaColor) -> Self {
        Self(
            (pixel.alpha as u32) << 24
                | ((pixel.red as u32) << 16)
                | ((pixel.green as u32) << 8)
                | (pixel.blue as u32),
        )
    }
}

#[cfg(feature = "software")]
impl From<SoftBufferPixel> for PremultipliedRgbaColor {
    #[inline]
    fn from(pixel: SoftBufferPixel) -> Self {
        let v = pixel.0;
        PremultipliedRgbaColor {
            red: (v >> 16) as u8,
            green: (v >> 8) as u8,
            blue: (v >> 0) as u8,
            alpha: (v >> 24) as u8,
        }
    }
}

#[cfg(feature = "software")]
unsafe impl AnyBitPattern for SoftBufferPixel {}
#[cfg(feature = "software")]
unsafe impl NoUninit for SoftBufferPixel {}

// ---------- BaseviewWindowWrapper ---------- //

pub(crate) struct BaseviewWindowWrapper {
    raw_display_handle: raw_window_handle::RawDisplayHandle,
    raw_window_handle: raw_window_handle::RawWindowHandle,
}

impl BaseviewWindowWrapper {
    fn new<'a>(window: &baseview::Window<'a>) -> Self {
        Self {
            raw_display_handle: window.display_handle().expect("No display handle").as_raw(),
            raw_window_handle: window.window_handle().expect("No window handle").as_raw(),
        }
    }
}

impl HasDisplayHandle for BaseviewWindowWrapper {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        unsafe {
            Ok(raw_window_handle::DisplayHandle::borrow_raw(
                self.raw_display_handle,
            ))
        }
    }
}

impl HasWindowHandle for BaseviewWindowWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        unsafe {
            Ok(raw_window_handle::WindowHandle::borrow_raw(
                self.raw_window_handle,
            ))
        }
    }
}

unsafe impl Send for BaseviewWindowWrapper {}
unsafe impl Sync for BaseviewWindowWrapper {}
