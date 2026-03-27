use baseview::{Event, EventStatus, MouseButton, MouseEvent, ScrollDelta};
use bitflags::bitflags;
use i_slint_common::for_each_special_keys;
use i_slint_core::{
    InternalToken,
    api::{LogicalPosition, LogicalSize, PhysicalSize, Window},
    items::PointerEventButton,
    menus::MenuVTable,
    platform::{WindowEvent, update_timers_and_animations},
    renderer::Renderer,
    window::{InputMethodRequest, WindowAdapter, WindowAdapterInternal},
};
use keyboard_types::{KeyState, Modifiers};
use std::{cell::RefCell, rc::Rc, str::FromStr};
use vtable::VRc;

use crate::{
    SbLogicalSize, SbRendererType,
    muda::{ID_SEP, MudaAdapter, MudaType},
    renderer::SbRendererAdapter,
};

// ---------- SbWindowAdapter ---------- //

pub struct SbWindowAdapter {
    inner: RefCell<SbWindowAdapterInner>,
    renderer_adapter: Box<dyn SbRendererAdapter>,
    window: Window,
    window_handle: raw_window_handle::RawWindowHandle,
    display_handle: raw_window_handle::RawDisplayHandle,
}

impl SbWindowAdapter {
    const LINE_PX: f32 = 60.0;

    pub(crate) fn new(
        size: SbLogicalSize,
        system_scale_factor: Option<f64>,
        user_scale_factor: f64,
        renderer_type: SbRendererType,
        window_handle: raw_window_handle::RawWindowHandle,
        display_handle: raw_window_handle::RawDisplayHandle,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| {
            let window = Window::new(this.clone() as _);

            let system_scale_factor = system_scale_factor.unwrap_or(1.0);
            let scale_factor = (user_scale_factor * system_scale_factor) as f32;
            window.dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor });

            Self {
                inner: RefCell::new(SbWindowAdapterInner {
                    size,
                    system_scale_factor,
                    user_scale_factor,
                    mouse_pos: LogicalPosition::new(0.0, 0.0),

                    cursor: None,
                    flags: Flags::empty(),
                    pending_user_scale_factor: None,

                    muda_menubar: None,
                    muda_context: None,
                }),
                renderer_adapter: renderer_type.create_adapter(),
                window,
                window_handle,
                display_handle,
            }
        })
    }

    // ---------- Public ---------- //

    pub fn set_user_scale_factor(&self, user_scale_factor: f64) -> bool {
        let mut inner = self.inner.borrow_mut();

        let current_scale_factor = inner
            .pending_user_scale_factor
            .unwrap_or(inner.user_scale_factor);
        if current_scale_factor == user_scale_factor {
            false
        } else {
            inner.pending_user_scale_factor.replace(user_scale_factor);
            true
        }
    }

    // ---------- Setter ---------- //

    pub(crate) fn set_window(&self, window: &baseview::Window) {
        if let Err(err) = self.renderer_adapter.set_window(window, self) {
            println!("Renderer set window error: {err}");
        }
    }

    // ---------- Events ---------- //

    pub(crate) fn on_frame(&self, window: &mut baseview::Window) {
        self.process_messages(window);
        update_timers_and_animations();

        if !self.inner.borrow_mut().contains_remove_flags(Flags::REDRAW) {
            return;
        }

        if let Err(err) = self.renderer_adapter.render(self) {
            println!("{err}");
        }
    }

    pub(crate) fn on_event(&self, window: &mut baseview::Window, event: Event) -> EventStatus {
        match event {
            Event::Mouse(event) => match event {
                MouseEvent::CursorMoved {
                    position,
                    modifiers,
                } => {
                    self.send_modifiers(modifiers);

                    let mouse_pos = {
                        let mut inner = self.inner.borrow_mut();
                        let scale_factor = inner.total_scale_factor();
                        inner.mouse_pos = LogicalPosition::new(
                            (position.x / scale_factor) as _,
                            (position.y / scale_factor) as _,
                        );
                        inner.mouse_pos
                    };

                    self.window.dispatch_event(WindowEvent::PointerMoved {
                        position: mouse_pos,
                    });
                }
                MouseEvent::ButtonPressed { button, modifiers } => {
                    self.send_modifiers(modifiers);

                    let mouse_pos = {
                        let mut inner = self.inner.borrow_mut();
                        inner.flags.insert(Flags::MOUSE_DOWN);
                        inner.mouse_pos
                    };

                    self.window.dispatch_event(WindowEvent::PointerPressed {
                        position: mouse_pos,
                        button: Self::convert_button(button),
                    });
                }
                MouseEvent::ButtonReleased { button, modifiers } => {
                    self.send_modifiers(modifiers);

                    let (mouse_pos, exit) = {
                        let mut inner = self.inner.borrow_mut();
                        inner.flags.remove(Flags::MOUSE_DOWN);

                        (
                            inner.mouse_pos,
                            inner.contains_remove_flags(Flags::PENDING_MOUSE_EXIT),
                        )
                    };

                    self.window.dispatch_event(WindowEvent::PointerReleased {
                        position: mouse_pos,
                        button: Self::convert_button(button),
                    });

                    if exit {
                        self.window.dispatch_event(WindowEvent::PointerExited);
                    }
                }
                MouseEvent::WheelScrolled { delta, modifiers } => {
                    self.send_modifiers(modifiers);

                    let position = self.inner.borrow().mouse_pos;
                    let (delta_x, delta_y) = match delta {
                        ScrollDelta::Lines { x, y } => (x * Self::LINE_PX, y * Self::LINE_PX),
                        ScrollDelta::Pixels { x, y } => (x, y),
                    };

                    self.window.dispatch_event(WindowEvent::PointerScrolled {
                        position,
                        delta_x,
                        delta_y,
                    });
                }
                MouseEvent::CursorLeft => {
                    let dispatch = {
                        let mut inner = self.inner.borrow_mut();
                        if inner.flags.contains(Flags::MOUSE_DOWN) {
                            inner.flags.insert(Flags::PENDING_MOUSE_EXIT);
                            false
                        } else {
                            true
                        }
                    };
                    if dispatch {
                        self.window.dispatch_event(WindowEvent::PointerExited);
                    }
                }
                _ => return baseview::EventStatus::Ignored,
            },
            Event::Keyboard(event) => {
                self.send_modifiers(event.modifiers);

                let text = event.key.to_string();
                macro_rules! modifier_to_char {
                    ($($char:literal # $name:ident # $($qt:ident)|* # $($winit:ident $(($_pos:ident))?)|* # $($xkb:ident)|* ;)*) => {
                        if false { unimplemented!() }
                        $($(
                            else if text == stringify!($winit) {
                                $char.into()
                            }
                        )*)*
                        else {
                            text.into()
                        }
                    };
                }
                let text = for_each_special_keys!(modifier_to_char);

                match event.state {
                    KeyState::Down => {
                        if event.repeat {
                            self.window
                                .dispatch_event(WindowEvent::KeyPressRepeated { text });
                        } else {
                            self.window.dispatch_event(WindowEvent::KeyPressed { text });
                        }
                    }
                    KeyState::Up => {
                        self.window
                            .dispatch_event(WindowEvent::KeyReleased { text });
                    }
                }
            }
            Event::Window(event) => match event {
                baseview::WindowEvent::Resized(info) => {
                    let (size, scale_factor) = {
                        let mut inner = self.inner.borrow_mut();

                        let scale_factor =
                            if let Some(scale_factor) = inner.pending_user_scale_factor.take() {
                                inner.user_scale_factor = scale_factor;
                                inner.total_scale_factor()
                            } else {
                                inner.system_scale_factor = info.scale() as _;

                                let scale_factor = inner.total_scale_factor();
                                let size = info.physical_size();
                                inner.size = SbLogicalSize::new(
                                    size.width as f64 / scale_factor,
                                    size.height as f64 / scale_factor,
                                );
                                scale_factor
                            };

                        (
                            LogicalSize::new(inner.size.width as _, inner.size.height as _),
                            scale_factor as f32,
                        )
                    };

                    if self.window.scale_factor() != scale_factor {
                        self.window
                            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor });
                    }

                    self.window.dispatch_event(WindowEvent::Resized { size });
                }
                baseview::WindowEvent::Focused => {
                    self.window
                        .dispatch_event(WindowEvent::WindowActiveChanged(true));
                }
                baseview::WindowEvent::Unfocused => {
                    self.window
                        .dispatch_event(WindowEvent::WindowActiveChanged(false));
                }
                baseview::WindowEvent::WillClose => {
                    self.window.dispatch_event(WindowEvent::CloseRequested);
                }
            },
        }

        self.process_messages(window);
        EventStatus::Captured
    }

    // ---------- Util ---------- //

    fn convert_button(button: MouseButton) -> PointerEventButton {
        match button {
            MouseButton::Left => PointerEventButton::Left,
            MouseButton::Middle => PointerEventButton::Middle,
            MouseButton::Right => PointerEventButton::Right,
            MouseButton::Back => PointerEventButton::Back,
            MouseButton::Forward => PointerEventButton::Forward,
            MouseButton::Other(_) => PointerEventButton::Other,
        }
    }

    fn convert_cursor(cursor: i_slint_core::items::MouseCursor) -> baseview::MouseCursor {
        match cursor {
            i_slint_core::items::MouseCursor::Crosshair => baseview::MouseCursor::Crosshair,
            i_slint_core::items::MouseCursor::Default => baseview::MouseCursor::Default,
            i_slint_core::items::MouseCursor::None => baseview::MouseCursor::Hidden,
            i_slint_core::items::MouseCursor::Help => baseview::MouseCursor::Help,
            i_slint_core::items::MouseCursor::Pointer => baseview::MouseCursor::Ptr,
            i_slint_core::items::MouseCursor::Progress => baseview::MouseCursor::Working,
            i_slint_core::items::MouseCursor::Wait => baseview::MouseCursor::Working,
            i_slint_core::items::MouseCursor::Text => baseview::MouseCursor::Text,
            i_slint_core::items::MouseCursor::Alias => baseview::MouseCursor::Alias,
            i_slint_core::items::MouseCursor::Copy => baseview::MouseCursor::Copy,
            i_slint_core::items::MouseCursor::Move => baseview::MouseCursor::Move,
            i_slint_core::items::MouseCursor::NoDrop => baseview::MouseCursor::PtrNotAllowed,
            i_slint_core::items::MouseCursor::NotAllowed => baseview::MouseCursor::NotAllowed,
            i_slint_core::items::MouseCursor::Grab => baseview::MouseCursor::Hand,
            i_slint_core::items::MouseCursor::Grabbing => baseview::MouseCursor::HandGrabbing,
            i_slint_core::items::MouseCursor::ColResize => baseview::MouseCursor::ColResize,
            i_slint_core::items::MouseCursor::RowResize => baseview::MouseCursor::RowResize,
            i_slint_core::items::MouseCursor::NResize => baseview::MouseCursor::NResize,
            i_slint_core::items::MouseCursor::EResize => baseview::MouseCursor::EResize,
            i_slint_core::items::MouseCursor::SResize => baseview::MouseCursor::SResize,
            i_slint_core::items::MouseCursor::WResize => baseview::MouseCursor::WResize,
            i_slint_core::items::MouseCursor::NeResize => baseview::MouseCursor::NeResize,
            i_slint_core::items::MouseCursor::NwResize => baseview::MouseCursor::NwResize,
            i_slint_core::items::MouseCursor::SeResize => baseview::MouseCursor::SeResize,
            i_slint_core::items::MouseCursor::SwResize => baseview::MouseCursor::SwResize,
            i_slint_core::items::MouseCursor::EwResize => baseview::MouseCursor::EwResize,
            i_slint_core::items::MouseCursor::NsResize => baseview::MouseCursor::NsResize,
            i_slint_core::items::MouseCursor::NeswResize => baseview::MouseCursor::NeswResize,
            i_slint_core::items::MouseCursor::NwseResize => baseview::MouseCursor::NwseResize,
            _ => baseview::MouseCursor::Default,
        }
    }

    // Swap control and meta according to slint's docs.
    fn convert_modifier(modifier: Modifiers) -> &'static str {
        if modifier == Modifiers::ALT {
            "\u{0012}"
        } else if modifier == Modifiers::ALT_GRAPH {
            "\u{0013}"
        } else if modifier == Modifiers::CAPS_LOCK {
            "\u{0014}"
        } else if modifier == Modifiers::CONTROL {
            "\u{0017}"
        } else if modifier == Modifiers::META {
            "\u{0011}"
        } else if modifier == Modifiers::SCROLL_LOCK {
            "\u{F72F}"
        } else if modifier == Modifiers::SHIFT {
            "\u{0010}"
        } else if modifier == Modifiers::SUPER {
            "\u{0017}"
        } else {
            ""
        }
    }

    fn process_messages(&self, window: &mut baseview::Window) {
        let (cursor, focus, size) = {
            let mut inner = self.inner.borrow_mut();

            for menu_event in muda::MenuEvent::receiver().try_iter() {
                let (muda_type, menu_index) = menu_event.id().0.split_once(ID_SEP).unwrap();
                let menu_index = menu_index.parse().unwrap();

                match MudaType::from_str(muda_type).unwrap() {
                    MudaType::Menubar => {
                        if let Some(muda_menubar) = inner.muda_menubar.as_ref() {
                            muda_menubar.invoke(menu_index);
                        }
                    }
                    MudaType::Context => {
                        if let Some(muda_context) = inner.muda_context.as_ref() {
                            muda_context.invoke(menu_index);
                        }
                    }
                }
            }

            (
                inner.cursor.take(),
                inner.contains_remove_flags(Flags::FOCUS),
                inner.pending_user_scale_factor.map(|scale_factor| {
                    let size = inner.size.to_physical(scale_factor);
                    baseview::Size {
                        width: size.width as _,
                        height: size.height as _,
                    }
                }),
            )
        };

        if let Some(cursor) = cursor {
            window.set_mouse_cursor(Self::convert_cursor(cursor));
        }

        if focus {
            window.focus();
        }

        if let Some(size) = size {
            window.resize(size);
        }
    }

    fn send_modifiers(&self, modifiers: Modifiers) {
        for modifier in [
            Modifiers::ALT,
            Modifiers::ALT_GRAPH,
            Modifiers::CAPS_LOCK,
            Modifiers::CONTROL,
            Modifiers::FN,
            Modifiers::FN_LOCK,
            Modifiers::META,
            Modifiers::NUM_LOCK,
            Modifiers::SCROLL_LOCK,
            Modifiers::SHIFT,
            Modifiers::SYMBOL,
            Modifiers::SYMBOL_LOCK,
            Modifiers::HYPER,
            Modifiers::SUPER,
        ] {
            if !modifiers.contains(modifier) {
                continue;
            }

            let text = Self::convert_modifier(modifier);
            if text.is_empty() {
                continue;
            }
            self.window
                .dispatch_event(WindowEvent::KeyPressed { text: text.into() });
        }
    }
}

// TODO: impl complete trait.
impl WindowAdapter for SbWindowAdapter {
    fn window(&self) -> &Window {
        &self.window
    }

    fn size(&self) -> PhysicalSize {
        let inner = self.inner.borrow();
        let size = inner.size.to_physical(inner.total_scale_factor());
        PhysicalSize::new(size.width, size.height)
    }

    fn request_redraw(&self) {
        self.inner.borrow_mut().flags.insert(Flags::REDRAW);
    }

    fn renderer(&self) -> &dyn Renderer {
        self.renderer_adapter.renderer()
    }

    fn internal(&self, _: InternalToken) -> Option<&dyn WindowAdapterInternal> {
        Some(self)
    }

    fn window_handle_06(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        unsafe {
            Ok(raw_window_handle::WindowHandle::borrow_raw(
                self.window_handle,
            ))
        }
    }

    fn display_handle_06(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        unsafe {
            Ok(raw_window_handle::DisplayHandle::borrow_raw(
                self.display_handle,
            ))
        }
    }
}

// TODO: impl complete trait.
impl WindowAdapterInternal for SbWindowAdapter {
    fn set_mouse_cursor(&self, cursor: i_slint_core::items::MouseCursor) {
        self.inner.borrow_mut().cursor.replace(cursor);
    }

    fn input_method_request(&self, request: InputMethodRequest) {
        match request {
            InputMethodRequest::Enable(_) => self.inner.borrow_mut().flags.insert(Flags::FOCUS),
            _ => (),
        }
    }

    #[cfg(feature = "native-menu-bar")]
    fn supports_native_menu_bar(&self) -> bool {
        true
    }

    #[cfg(feature = "native-menu-bar")]
    fn setup_menubar(&self, menubar: VRc<MenuVTable>) {
        let mut inner = self.inner.borrow_mut();
        // On Windows, we must destroy the muda menu before re-creating a new one.
        drop(inner.muda_menubar.take());
        inner
            .muda_menubar
            .replace(MudaAdapter::setup(menubar, MudaType::Menubar));
    }

    fn show_native_popup_menu(
        &self,
        context_menu_item: VRc<MenuVTable>,
        position: LogicalPosition,
    ) -> bool {
        let mut inner = self.inner.borrow_mut();
        // On Windows, we must destroy the muda menu before re-creating a new one.
        drop(inner.muda_context.take());
        inner.muda_context = MudaAdapter::show_context_menu(
            context_menu_item,
            self.window_handle,
            position,
            inner.user_scale_factor,
        );
        inner.muda_context.is_some()
    }
}

// ---------- SbWindowAdapter ---------- //

struct SbWindowAdapterInner {
    size: SbLogicalSize,
    system_scale_factor: f64,
    user_scale_factor: f64,
    mouse_pos: LogicalPosition,

    cursor: Option<i_slint_core::items::MouseCursor>,
    flags: Flags,
    pending_user_scale_factor: Option<f64>,

    muda_menubar: Option<MudaAdapter>,
    muda_context: Option<MudaAdapter>,
}

impl SbWindowAdapterInner {
    fn contains_remove_flags(&mut self, flags: Flags) -> bool {
        let value = self.flags.contains(flags);
        if value {
            self.flags.remove(flags);
        }
        value
    }

    fn total_scale_factor(&self) -> f64 {
        self.system_scale_factor * self.user_scale_factor
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    struct Flags: u8 {
        const FOCUS = 1 << 0;
        const MOUSE_DOWN = 1 << 1;
        const PENDING_MOUSE_EXIT = 1 << 2;
        const REDRAW = 1 << 3;
    }
}
