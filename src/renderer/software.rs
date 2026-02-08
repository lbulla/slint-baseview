use bytemuck::{AnyBitPattern, NoUninit, Zeroable, cast_slice_mut};
use i_slint_core::{renderer::Renderer, window::WindowAdapter};
use i_slint_renderer_software::{PremultipliedRgbaColor, SoftwareRenderer, TargetPixel};
use std::{cell::RefCell, num::NonZeroU32, ops::DerefMut, sync::Arc};
use softbuffer::{Context, Surface};

use super::{SbDisplayWindowHandle, SbRendererAdapter};

// ---------- SbSoftwareRendererAdapter ---------- //

#[derive(Default)]
pub(super) struct SbSoftwareRendererAdapter {
    renderer: SoftwareRenderer,
    context: RefCell<Option<Context<Arc<SbDisplayWindowHandle>>>>,
    surface: RefCell<Option<Surface<Arc<SbDisplayWindowHandle>, Arc<SbDisplayWindowHandle>>>>,
}

impl SbRendererAdapter for SbSoftwareRendererAdapter {
    fn set_window(
        &self,
        window: &baseview::Window,
        _window_adapter: &dyn WindowAdapter,
    ) -> Result<(), String> {
        let window_wrapper = Arc::new(SbDisplayWindowHandle::new(window));
        let context = Context::new(window_wrapper.clone())
            .map_err(|err| format!("Software context error: {err}"))?;
        let surface = Surface::new(&context, window_wrapper)
            .map_err(|err| format!("Software surface error: {err}"))?;
        self.context.borrow_mut().replace(context);
        self.surface.borrow_mut().replace(surface);
        Ok(())
    }

    fn render(&self, window_adapter: &dyn WindowAdapter) -> Result<(), String> {
        let mut surface = self.surface.borrow_mut();
        let Some(surface) = surface.as_mut() else {
            return Ok(());
        };

        let size = window_adapter.size();
        surface
            .resize(
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            )
            .map_err(|err| format!("Software resize error: {err}"))?;

        let mut buffer = surface
            .buffer_mut()
            .map_err(|err| format!("Software buffer error: {err}"))?;

        let soft_buffer: &mut [SbPixel] = cast_slice_mut(buffer.deref_mut());
        self.renderer.render(soft_buffer, size.width as _);
        buffer
            .present()
            .map_err(|err| format!("Software present error: {err}"))
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}

// ---------- SbPixel ---------- //

#[derive(Clone, Copy, Zeroable)]
struct SbPixel(u32);

impl TargetPixel for SbPixel {
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

impl From<PremultipliedRgbaColor> for SbPixel {
    fn from(pixel: PremultipliedRgbaColor) -> Self {
        Self(
            (pixel.alpha as u32) << 24
                | ((pixel.red as u32) << 16)
                | ((pixel.green as u32) << 8)
                | (pixel.blue as u32),
        )
    }
}

impl From<SbPixel> for PremultipliedRgbaColor {
    #[inline]
    fn from(pixel: SbPixel) -> Self {
        let v = pixel.0;
        PremultipliedRgbaColor {
            red: (v >> 16) as u8,
            green: (v >> 8) as u8,
            blue: (v >> 0) as u8,
            alpha: (v >> 24) as u8,
        }
    }
}

unsafe impl AnyBitPattern for SbPixel {}
unsafe impl NoUninit for SbPixel {}
