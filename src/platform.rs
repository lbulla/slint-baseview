use baseview::{copy_to_clipboard, paste_from_clipboard};
use i_slint_core::{
    api::PlatformError,
    platform::{Clipboard, Platform},
    window::WindowAdapter,
};
use std::{cell::RefCell, rc::Rc};

use crate::SbWindowAdapter;

// ---------- SbPlatform ---------- //

#[derive(Default)]
pub(crate) struct SbPlatform {}

impl SbPlatform {
    thread_local! {
        pub(crate) static WINDOW_ADAPTER: RefCell<Option<Rc<SbWindowAdapter>>> = Default::default();
    }
}

// TODO: impl complete trait.
impl Platform for SbPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Self::WINDOW_ADAPTER.with_borrow_mut(|a| match a.take() {
            Some(a) => Ok(a as _),
            None => Err(PlatformError::Other("No `WINDOW_ADAPTER_INNER`".into())),
        })
    }

    fn set_clipboard_text(&self, text: &str, clipboard: Clipboard) {
        if let Err(err) = copy_to_clipboard(text, clipboard == Clipboard::SelectionClipboard) {
            eprintln!("Failed to set clipboard text: {err}");
        }
    }

    fn clipboard_text(&self, clipboard: Clipboard) -> Option<String> {
        match paste_from_clipboard(clipboard == Clipboard::SelectionClipboard) {
            Ok(text) => Some(text),
            Err(err) => {
                eprintln!("Failed to get clipboard text: {err}");
                None
            }
        }
    }
}
