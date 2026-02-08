use baseview::{Event, EventStatus, WindowOpenOptions, WindowScalePolicy};
use i_slint_core::platform::set_platform;
use raw_window_handle::{HandleError, HasWindowHandle};
use std::rc::Rc;

use crate::{Receiver, SbLogicalSize, SbRendererType, SbWindowAdapter, platform::SbPlatform};

// ---------- SbWindow ---------- //

pub struct SbWindow {
    handle: baseview::WindowHandle,
}

impl SbWindow {
    pub fn open<B, E, P>(
        parent: P,
        title: String,
        size: SbLogicalSize,
        system_scale_factor: Option<f64>,
        user_scale_factor: f64,
        renderer_type: SbRendererType,
        receiver: Receiver<E::Message>,
        build: B,
    ) -> Self
    where
        B: FnOnce(Rc<SbWindowAdapter>) -> E + Send + Sync + 'static,
        E: SbExecutor + 'static,
        P: HasWindowHandle,
    {
        let user_size = size.to_physical(user_scale_factor);
        let handle = baseview::Window::open_parented(
            parent,
            WindowOpenOptions {
                title,
                size: baseview::Size::new(user_size.width as _, user_size.height as _),
                scale: system_scale_factor
                    .map(|s| WindowScalePolicy::ScaleFactor(s as _))
                    .unwrap_or(WindowScalePolicy::SystemScaleFactor),
            },
            move |window| {
                set_platform(Box::new(SbPlatform::default())).unwrap();

                let window_adapter = SbWindowAdapter::new(
                    size,
                    system_scale_factor,
                    user_scale_factor,
                    renderer_type,
                );
                SbPlatform::WINDOW_ADAPTER.with_borrow_mut(|a| a.replace(window_adapter.clone()));
                window_adapter.set_window(window);

                SbWindowHandler {
                    executor: build(window_adapter.clone()),
                    receiver,
                    window_adapter,
                }
            },
        );

        Self { handle }
    }

    pub fn close(&mut self) {
        self.handle.close();
    }

    pub fn is_open(&self) -> bool {
        self.handle.is_open()
    }
}

impl HasWindowHandle for SbWindow {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, HandleError> {
        self.handle.window_handle()
    }
}

// ---------- SbExecutor ---------- //

pub trait SbExecutor {
    type Message: Send;

    fn on_frame(&mut self);
    fn on_event(&mut self, event: &Event) -> EventStatus;
    fn on_message(&mut self, task: Self::Message);
}

// ---------- SbWindowHandler ---------- //

struct SbWindowHandler<E: SbExecutor> {
    executor: E,
    receiver: Receiver<E::Message>,
    window_adapter: Rc<SbWindowAdapter>,
}

impl<E: SbExecutor> SbWindowHandler<E> {
    fn process_messages(&mut self) {
        for message in self.receiver.try_iter() {
            self.executor.on_message(message);
        }
    }
}

impl<E: SbExecutor> baseview::WindowHandler for SbWindowHandler<E> {
    fn on_frame(&mut self, window: &mut baseview::Window) {
        self.process_messages();
        self.executor.on_frame();
        self.window_adapter.on_frame(window);
    }

    fn on_event(&mut self, window: &mut baseview::Window, event: Event) -> EventStatus {
        let mut status = self.executor.on_event(&event);
        if status == EventStatus::Ignored {
            status = self.window_adapter.on_event(window, event);
        }
        self.process_messages();
        status
    }
}
