use i_slint_common::for_each_special_keys;
use i_slint_core::{
    api::{LogicalPosition, LogicalSize, PhysicalSize, Window},
    items::PointerEventButton,
    platform::{WindowEvent, update_timers_and_animations},
    renderer::Renderer,
    window::WindowAdapter,
};
use std::{cell::RefCell, rc::Rc};

use crate::{EmbeddedRendererType, renderer::EmbeddedRendererAdapter};

#[cfg(feature = "femtovg")]
use crate::renderer::EmbeddedFemtoVGRendererAdapter;

#[cfg(feature = "skia")]
use crate::renderer::EmbeddedSkiaRendererAdapter;

#[cfg(feature = "software")]
use crate::renderer::EmbeddedSoftwareRendererAdapter;

// ---------- EmbeddedWindowAdapter ---------- //

pub(crate) struct EmbeddedWindowAdapter {
    inner: RefCell<EmbeddedWindowAdapterInner>,
    slint_window: Window,
    renderer_adapter: Box<dyn EmbeddedRendererAdapter>,
}

impl EmbeddedWindowAdapter {
    const LINE_PX: f32 = 60.0;

    pub(crate) fn new(
        size: LogicalSize,
        user_scale_factor: f32,
        system_scale_policy: baseview::WindowScalePolicy,
        renderer_type: EmbeddedRendererType,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| {
            let slint_window = Window::new(this.clone() as _);
            slint_window.dispatch_event(WindowEvent::ScaleFactorChanged {
                scale_factor: user_scale_factor,
            });

            let renderer_adapter: Box<dyn EmbeddedRendererAdapter> = match renderer_type {
                #[cfg(feature = "femtovg")]
                EmbeddedRendererType::FemtoVG => {
                    Box::new(EmbeddedFemtoVGRendererAdapter::default())
                }
                #[cfg(feature = "skia")]
                EmbeddedRendererType::Skia => Box::new(EmbeddedSkiaRendererAdapter::default()),
                #[cfg(feature = "software")]
                EmbeddedRendererType::Software => {
                    Box::new(EmbeddedSoftwareRendererAdapter::default())
                }
            };

            Self {
                inner: RefCell::new(EmbeddedWindowAdapterInner {
                    size,
                    system_scale_factor: match system_scale_policy {
                        baseview::WindowScalePolicy::SystemScaleFactor => 1.0,
                        baseview::WindowScalePolicy::ScaleFactor(s) => s as _,
                    },
                    user_scale_factor,
                    mouse_pos: LogicalPosition::new(0.0, 0.0),
                    mouse_down: false,
                    pending_mouse_exit: false,
                }),
                slint_window,
                renderer_adapter,
            }
        })
    }

    // ---------- Getter ---------- //

    pub(crate) fn renderer(&self) -> &dyn Renderer {
        self.renderer_adapter.renderer()
    }

    // ---------- Setter ---------- //

    pub(crate) fn set_window(&self, baseview_window: &baseview::Window) {
        if let Err(err) = self
            .renderer_adapter
            .set_window(baseview_window, &self.slint_window)
        {
            println!("{err}");
        }
    }

    pub(crate) fn set_user_scale_factor(&self, user_scale_factor: f32) {
        let physical_size = {
            let mut inner = self.inner.borrow_mut();
            inner.user_scale_factor = user_scale_factor;
            inner.physical_size()
        };

        // TODO: Trigger resize.
        if let Err(err) = self.renderer_adapter.renderer().resize(physical_size) {
            println!("{err}");
        }

        self.slint_window
            .dispatch_event(WindowEvent::ScaleFactorChanged {
                scale_factor: user_scale_factor,
            });
    }

    // ---------- Events ---------- //

    pub(crate) fn on_frame(&self) {
        update_timers_and_animations();

        if let Err(err) = self.renderer_adapter.render(&self.slint_window) {
            println!("{err}");
        }
    }

    pub(crate) fn on_event(&self, event: baseview::Event) -> baseview::EventStatus {
        match event {
            baseview::Event::Mouse(mouse_event) => match mouse_event {
                baseview::MouseEvent::CursorMoved {
                    position,
                    modifiers,
                } => {
                    self.send_modifiers(modifiers);

                    let mouse_pos = {
                        let mut inner = self.inner.borrow_mut();
                        inner.mouse_pos = LogicalPosition::new(
                            position.x as f32 / inner.user_scale_factor,
                            position.y as f32 / inner.user_scale_factor,
                        );
                        inner.mouse_pos
                    };
                    self.slint_window.dispatch_event(WindowEvent::PointerMoved {
                        position: mouse_pos,
                    });
                }
                baseview::MouseEvent::ButtonPressed { button, modifiers } => {
                    self.send_modifiers(modifiers);

                    let mouse_pos = {
                        let mut inner = self.inner.borrow_mut();
                        inner.mouse_down = true;
                        inner.mouse_pos
                    };
                    self.slint_window
                        .dispatch_event(WindowEvent::PointerPressed {
                            position: mouse_pos,
                            button: Self::convert_button(button),
                        });
                }
                baseview::MouseEvent::ButtonReleased { button, modifiers } => {
                    self.send_modifiers(modifiers);

                    let (mouse_pos, exit) = {
                        let mut inner = self.inner.borrow_mut();
                        let exit = inner.pending_mouse_exit;
                        inner.mouse_down = false;
                        (inner.mouse_pos, exit)
                    };
                    self.slint_window
                        .dispatch_event(WindowEvent::PointerReleased {
                            position: mouse_pos,
                            button: Self::convert_button(button),
                        });

                    if exit {
                        self.slint_window.dispatch_event(WindowEvent::PointerExited);
                    }
                }
                baseview::MouseEvent::WheelScrolled { delta, modifiers } => {
                    self.send_modifiers(modifiers);

                    let (delta_x, delta_y) = match delta {
                        baseview::ScrollDelta::Lines { x, y } => {
                            (x * Self::LINE_PX, y * Self::LINE_PX)
                        }
                        baseview::ScrollDelta::Pixels { x, y } => (x, y),
                    };
                    self.slint_window
                        .dispatch_event(WindowEvent::PointerScrolled {
                            position: self.inner.borrow().mouse_pos,
                            delta_x,
                            delta_y,
                        });
                }
                baseview::MouseEvent::CursorLeft => {
                    let mut inner = self.inner.borrow_mut();
                    if inner.mouse_down {
                        inner.pending_mouse_exit = true;
                    } else {
                        self.slint_window.dispatch_event(WindowEvent::PointerExited);
                    }
                }
                _ => return baseview::EventStatus::Ignored,
            },
            baseview::Event::Keyboard(key_event) => {
                self.send_modifiers(key_event.modifiers);

                let text = key_event.key.to_string();
                macro_rules! modifier_to_char {
                    ($($char:literal # $name:ident # $($qt:ident)|* # $($winit:ident $(($_pos:ident))?)|* # $($xkb:ident)|* ;)*) => {
                        if false { unimplemented!() }

                        $($(
                            else if text == stringify!($winit) {
                                $char.into()
                            }
                        )*)*

                        else {
                            text
                        }
                    };
                }
                let text = for_each_special_keys!(modifier_to_char).into();

                match key_event.state {
                    keyboard_types::KeyState::Down => {
                        if key_event.repeat {
                            self.slint_window
                                .dispatch_event(WindowEvent::KeyPressRepeated { text });
                        } else {
                            self.slint_window
                                .dispatch_event(WindowEvent::KeyPressed { text });
                        }
                    }
                    keyboard_types::KeyState::Up => {
                        self.slint_window
                            .dispatch_event(WindowEvent::KeyReleased { text });
                    }
                }
            }
            baseview::Event::Window(window_event) => match window_event {
                baseview::WindowEvent::Resized(info) => {
                    let (logical, physical) = {
                        let mut inner = self.inner.borrow_mut();
                        let logical = info.logical_size();
                        inner.size = LogicalSize::new(logical.width as _, logical.height as _);
                        inner.system_scale_factor = info.scale() as _;
                        (inner.size, inner.physical_size())
                    };
                    if let Err(err) = self.renderer_adapter.renderer().resize(physical) {
                        println!("{err}");
                    }
                    self.slint_window
                        .dispatch_event(WindowEvent::Resized { size: logical });
                }
                baseview::WindowEvent::Focused => {
                    self.slint_window
                        .dispatch_event(WindowEvent::WindowActiveChanged(true));
                }
                baseview::WindowEvent::Unfocused => {
                    self.slint_window
                        .dispatch_event(WindowEvent::WindowActiveChanged(false));
                }
                baseview::WindowEvent::WillClose => {
                    self.slint_window
                        .dispatch_event(WindowEvent::CloseRequested);
                }
            },
        }
        baseview::EventStatus::Captured
    }

    // ---------- Util ---------- //

    fn convert_button(button: baseview::MouseButton) -> PointerEventButton {
        match button {
            baseview::MouseButton::Left => PointerEventButton::Left,
            baseview::MouseButton::Middle => PointerEventButton::Middle,
            baseview::MouseButton::Right => PointerEventButton::Right,
            baseview::MouseButton::Back => PointerEventButton::Back,
            baseview::MouseButton::Forward => PointerEventButton::Forward,
            baseview::MouseButton::Other(_) => PointerEventButton::Other,
        }
    }

    // Swap control and meta according to slint's docs.
    fn convert_modifier(modifier: keyboard_types::Modifiers) -> &'static str {
        if modifier == keyboard_types::Modifiers::ALT {
            "\u{0012}"
        } else if modifier == keyboard_types::Modifiers::ALT_GRAPH {
            "\u{0013}"
        } else if modifier == keyboard_types::Modifiers::CAPS_LOCK {
            "\u{0014}"
        } else if modifier == keyboard_types::Modifiers::CONTROL {
            "\u{0017}"
        } else if modifier == keyboard_types::Modifiers::META {
            "\u{0011}"
        } else if modifier == keyboard_types::Modifiers::SCROLL_LOCK {
            "\u{F72F}"
        } else if modifier == keyboard_types::Modifiers::SHIFT {
            "\u{0010}"
        } else if modifier == keyboard_types::Modifiers::SUPER {
            "\u{0017}"
        } else {
            ""
        }
    }

    fn send_modifiers(&self, modifiers: keyboard_types::Modifiers) {
        for modifier in [
            keyboard_types::Modifiers::ALT,
            keyboard_types::Modifiers::ALT_GRAPH,
            keyboard_types::Modifiers::CAPS_LOCK,
            keyboard_types::Modifiers::CONTROL,
            keyboard_types::Modifiers::FN,
            keyboard_types::Modifiers::FN_LOCK,
            keyboard_types::Modifiers::META,
            keyboard_types::Modifiers::NUM_LOCK,
            keyboard_types::Modifiers::SCROLL_LOCK,
            keyboard_types::Modifiers::SHIFT,
            keyboard_types::Modifiers::SYMBOL,
            keyboard_types::Modifiers::SYMBOL_LOCK,
            keyboard_types::Modifiers::HYPER,
            keyboard_types::Modifiers::SUPER,
        ] {
            if !modifiers.contains(modifier) {
                continue;
            }

            let text = Self::convert_modifier(modifier);
            if text.is_empty() {
                continue;
            }
            self.slint_window
                .dispatch_event(WindowEvent::KeyPressed { text: text.into() });
        }
    }
}

impl WindowAdapter for EmbeddedWindowAdapter {
    fn window(&self) -> &Window {
        &self.slint_window
    }

    fn size(&self) -> PhysicalSize {
        self.inner.borrow().physical_size()
    }

    fn renderer(&self) -> &dyn Renderer {
        self.renderer()
    }
}

// ---------- EmbeddedWindowAdapterInner ---------- //

struct EmbeddedWindowAdapterInner {
    size: LogicalSize,
    system_scale_factor: f32,
    user_scale_factor: f32,
    mouse_pos: LogicalPosition,
    mouse_down: bool,
    pending_mouse_exit: bool,
}

impl EmbeddedWindowAdapterInner {
    fn physical_size(&self) -> PhysicalSize {
        self.size.to_physical(self.scale())
    }

    fn scale(&self) -> f32 {
        self.system_scale_factor * self.user_scale_factor
    }
}
