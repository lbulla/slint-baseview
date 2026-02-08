use crossbeam_channel::Receiver;
use i_slint_core::{api::LogicalSize, platform::set_platform};
use raw_window_handle::{HandleError, HasWindowHandle, WindowHandle};
use std::{error::Error, path::Path, rc::Rc, sync::Arc};

use crate::{platform::EmbeddedPlatform, window_adapter::EmbeddedWindowAdapter};

// ---------- EmbeddedWindow ---------- //

pub enum EmbeddedRendererType {
    #[cfg(feature = "femtovg")]
    FemtoVG,
    #[cfg(feature = "skia")]
    Skia,
    #[cfg(feature = "software")]
    Software,
}

pub struct EmbeddedWindow {
    window_handle: baseview::WindowHandle,
}

impl EmbeddedWindow {
    pub fn new<B, M, V>(
        parent: impl HasWindowHandle,
        title: String,
        size: LogicalSize,
        user_scale_factor: f32,
        system_scale_policy: baseview::WindowScalePolicy,
        renderer_type: EmbeddedRendererType,
        receiver: Arc<Receiver<M>>,
        build: B,
    ) -> Self
    where
        B: Fn(EmbeddedWindowInterface) -> V + Send + 'static,
        M: Send + 'static,
        V: EmbeddedView<M> + 'static,
    {
        let window_handle = baseview::Window::open_parented(
            parent,
            baseview::WindowOpenOptions {
                title,
                size: baseview::Size::new(
                    (size.width * user_scale_factor) as _,
                    (size.height * user_scale_factor) as _,
                ),
                scale: system_scale_policy,
            },
            move |baseview_window| {
                let _ = set_platform(Box::new(EmbeddedPlatform::default()));

                let window_adapter = EmbeddedWindowAdapter::new(
                    size,
                    user_scale_factor,
                    system_scale_policy,
                    renderer_type,
                );
                EmbeddedPlatform::WINDOW_ADAPTER_INNER
                    .with_borrow_mut(|a| a.replace(window_adapter.clone()));
                window_adapter.set_window(baseview_window);

                let interface = EmbeddedWindowInterface {
                    window_adapter: window_adapter.clone(),
                };

                EmbeddedWindowHandler {
                    receiver,
                    view: build(interface),
                    window_adapter,
                }
            },
        );

        Self { window_handle }
    }

    pub fn close(&mut self) {
        self.window_handle.close();
    }

    pub fn is_open(&self) -> bool {
        self.window_handle.is_open()
    }
}

impl HasWindowHandle for EmbeddedWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.window_handle.window_handle()
    }
}

// ---------- EmbeddedView ---------- //

pub trait EmbeddedView<M: Send> {
    fn on_message(&self, message: M);
}

// ---------- EmbeddedWindowInterface ---------- //

pub struct EmbeddedWindowInterface {
    window_adapter: Rc<EmbeddedWindowAdapter>,
}

impl EmbeddedWindowInterface {
    pub fn register_font_from_memory(&self, data: &'static [u8]) -> Result<(), Box<dyn Error>> {
        self.window_adapter
            .renderer()
            .register_font_from_memory(data)
    }

    pub fn register_font_from_path(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.window_adapter.renderer().register_font_from_path(path)
    }

    pub fn set_user_scale_factor(&self, user_scale_factor: f32) {
        self.window_adapter.set_user_scale_factor(user_scale_factor);
    }
}

// ---------- EmbeddedWindowHandler ---------- //

struct EmbeddedWindowHandler<M: Send, V: EmbeddedView<M>> {
    receiver: Arc<Receiver<M>>,
    view: V,
    window_adapter: Rc<EmbeddedWindowAdapter>,
}

impl<E: Send, V: EmbeddedView<E>> baseview::WindowHandler for EmbeddedWindowHandler<E, V> {
    fn on_frame(&mut self, _window: &mut baseview::Window) {
        for message in self.receiver.try_iter() {
            self.view.on_message(message);
        }

        self.window_adapter.on_frame();
    }

    fn on_event(
        &mut self,
        _window: &mut baseview::Window,
        event: baseview::Event,
    ) -> baseview::EventStatus {
        self.window_adapter.on_event(event)
    }
}
